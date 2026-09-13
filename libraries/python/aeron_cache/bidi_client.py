"""
Bidirectional WebSocket transport for Aeron Cache.

A single WebSocket connection to ``/api/ws/v1/bidi`` carries the full cache + counter command
surface plus dynamic subscribe/unsubscribe, multiplexed by a client-minted ``correlationId`` that
the server echoes on every correlated frame. This is the JSON/WebSocket analogue of the Aeron
gateway transport, and an alternative to the HTTP+WS :class:`AeronCacheClient`.

The API is async (the connection is persistent); method names and response models mirror the
HTTP client.
"""

import asyncio
import json
import uuid

import websockets

from .models import (
    CreateResponse,
    PutItemResponse,
    GetItemResponse,
    DeleteItemResponse,
    DeleteCacheResponse,
    ClearCacheResponse,
    GetCacheResponse,
    GetCountersResponse,
    CacheItem,
    CounterItem,
    CounterResponse,
    PatchItemResponse,
    CancelItemRemovalResponse,
    StatEntry,
    CacheUpdateEvent,
    CounterUpdateEvent,
)


class BidiError(Exception):
    """An error frame correlated to a request."""

    def __init__(self, status, message):
        super().__init__(f"{status}: {message}")
        self.status = status
        self.message = message


class BidiSubscription:
    """Handle for an active subscription; ``await close()`` to unsubscribe."""

    def __init__(self, client, correlation_id, cache_id, counters):
        self._client = client
        self.correlation_id = correlation_id
        self.cache_id = cache_id
        self.counters = counters

    async def close(self):
        await self._client._unsubscribe(self.correlation_id, self.cache_id, self.counters)


class AeronBidiClient:
    def __init__(self, ws_url, request_timeout: float = 10.0):
        self.ws_url = ws_url.rstrip("/")
        self.request_timeout = request_timeout
        self._ws = None
        self._reader_task = None
        self._connect_lock = asyncio.Lock()
        # correlationId -> asyncio.Future for single-response commands
        self._pending = {}
        # correlationId -> {"items": {}, "cacheId": str, "status": str, "future": Future}
        self._entries = {}
        # correlationId -> {"stats": [], "status": str, "future": Future}
        self._stats = {}
        # correlationId -> Future resolving to the ack's cacheIds
        self._sub_acks = {}
        # cacheId -> list of {"cid", "on_message", "counters"}. Stream updates are routed by cacheId
        # because the server stamps streamUpdate.correlationId with the *causing command's* id
        # (the update's requestId), not the subscription's id.
        self._subs = {}

    # ------------------------------------------------------------------ lifecycle

    async def connect(self):
        if self._ws is not None:
            return
        async with self._connect_lock:
            if self._ws is not None:
                return
            self._ws = await websockets.connect(f"{self.ws_url}/api/ws/v1/bidi")
            self._reader_task = asyncio.create_task(self._read_loop())

    async def close(self):
        if self._reader_task is not None:
            self._reader_task.cancel()
            self._reader_task = None
        if self._ws is not None:
            await self._ws.close()
            self._ws = None
        self._fail_all(ConnectionError("AeronBidiClient closed"))

    async def __aenter__(self):
        await self.connect()
        return self

    async def __aexit__(self, *exc):
        await self.close()

    # ------------------------------------------------------------------ reader / dispatch

    async def _read_loop(self):
        try:
            async for raw in self._ws:
                try:
                    self._dispatch(json.loads(raw))
                except Exception:
                    pass
        except asyncio.CancelledError:
            raise
        except Exception as e:
            self._fail_all(e)

    def _dispatch(self, msg):
        msg_type = msg.get("type")
        cid = msg.get("correlationId")
        if msg_type == "commandResponse":
            fut = self._pending.pop(cid, None)
            if fut is not None and not fut.done():
                fut.set_result(msg)
        elif msg_type == "entries":
            acc = self._entries.get(cid)
            if acc is not None:
                acc["items"].update(msg.get("items") or {})
                acc["cacheId"] = msg.get("cacheId")
                acc["status"] = msg.get("status")
                if msg.get("endOfBatch"):
                    self._entries.pop(cid, None)
                    if not acc["future"].done():
                        acc["future"].set_result(acc)
        elif msg_type == "stats":
            acc = self._stats.get(cid)
            if acc is not None:
                acc["stats"].extend(msg.get("stats") or [])
                acc["status"] = msg.get("status")
                if msg.get("endOfBatch"):
                    self._stats.pop(cid, None)
                    if not acc["future"].done():
                        acc["future"].set_result(acc)
        elif msg_type == "subscribed":
            fut = self._sub_acks.pop(cid, None)
            if fut is not None and not fut.done():
                fut.set_result(msg.get("cacheIds") or [])
        elif msg_type == "streamUpdate":
            self._dispatch_stream_update(cid, msg)
        elif msg_type == "error":
            self._dispatch_error(cid, msg)

    def _dispatch_stream_update(self, cid, msg):
        cache_id = msg.get("cacheId")
        subs = self._subs.get(cache_id)
        if not subs:
            return
        value = msg.get("value")
        for sub in list(subs):
            on_message = sub["on_message"]
            if sub["counters"]:
                event = CounterUpdateEvent(
                    cacheId=cache_id,
                    eventType=msg.get("eventType"),
                    requestId=cid,
                    itemKey=msg.get("key"),
                    itemValue=(int(value) if value is not None else None),
                )
            else:
                event = CacheUpdateEvent(
                    cacheId=cache_id,
                    eventType=msg.get("eventType"),
                    requestId=cid,
                    itemKey=msg.get("key"),
                    itemValue=(str(value) if value is not None else None),
                )
            if asyncio.iscoroutinefunction(on_message):
                asyncio.create_task(on_message(event))
            else:
                on_message(event)

    def _dispatch_error(self, cid, msg):
        err = BidiError(msg.get("status"), msg.get("message"))
        if cid is None:
            return
        fut = self._pending.pop(cid, None)
        if fut is not None and not fut.done():
            fut.set_exception(err)
            return
        acc = self._entries.pop(cid, None) or self._stats.pop(cid, None) or self._sub_acks.pop(cid, None)
        if acc is not None:
            future = acc["future"] if isinstance(acc, dict) else acc
            if not future.done():
                future.set_exception(err)

    def _fail_all(self, exc):
        for fut in self._pending.values():
            if not fut.done():
                fut.set_exception(exc)
        self._pending.clear()
        for acc in list(self._entries.values()) + list(self._stats.values()):
            if not acc["future"].done():
                acc["future"].set_exception(exc)
        self._entries.clear()
        self._stats.clear()
        for fut in self._sub_acks.values():
            if not fut.done():
                fut.set_exception(exc)
        self._sub_acks.clear()

    # ------------------------------------------------------------------ send helpers

    async def _send(self, frame):
        await self.connect()
        await self._ws.send(json.dumps(frame))

    async def _command(self, op, cache_id=None, key=None, value=None, ttl=0, counter_value=0):
        cid = str(uuid.uuid4())
        fut = asyncio.get_event_loop().create_future()
        self._pending[cid] = fut
        await self._send({
            "type": "command", "correlationId": cid, "op": op,
            "cacheId": cache_id, "key": key, "value": value,
            "ttl": ttl, "counterValue": counter_value,
        })
        try:
            return await asyncio.wait_for(fut, self.request_timeout)
        finally:
            self._pending.pop(cid, None)

    async def _batched(self, op, cache_id, kind):
        cid = str(uuid.uuid4())
        fut = asyncio.get_event_loop().create_future()
        acc = {"items": {}, "stats": [], "cacheId": cache_id, "status": None, "future": fut}
        (self._entries if kind == "entries" else self._stats)[cid] = acc
        await self._send({
            "type": "command", "correlationId": cid, "op": op,
            "cacheId": cache_id, "key": None, "value": None, "ttl": 0, "counterValue": 0,
        })
        try:
            return await asyncio.wait_for(fut, self.request_timeout)
        finally:
            self._entries.pop(cid, None)
            self._stats.pop(cid, None)

    # ------------------------------------------------------------------ cache commands

    async def create_cache(self, cache_id) -> CreateResponse:
        r = await self._command("CREATE_CACHE", cache_id)
        return CreateResponse(cacheId=r.get("cacheId"), operationStatus=r.get("status"))

    async def put_item(self, cache_id, key, value) -> PutItemResponse:
        return await self.put_timed_item(cache_id, key, value, 0)

    async def put_timed_item(self, cache_id, key, value, ttl) -> PutItemResponse:
        r = await self._command("ADD_CACHE_ENTRY", cache_id, key, value, ttl=ttl)
        return PutItemResponse(cacheId=r.get("cacheId"), key=r.get("key"), operationStatus=r.get("status"))

    async def patch_item(self, cache_id, key, value) -> PatchItemResponse:
        r = await self._command("PATCH_CACHE_ENTRY", cache_id, key, value)
        return PatchItemResponse(cacheId=r.get("cacheId"), key=r.get("key"), operationStatus=r.get("status"))

    async def get_item(self, cache_id, key) -> GetItemResponse:
        r = await self._command("GET_CACHE_ENTRY", cache_id, key)
        return GetItemResponse(cacheId=r.get("cacheId"), key=r.get("key"), value=r.get("value"), operationStatus=r.get("status"))

    async def delete_item(self, cache_id, key) -> DeleteItemResponse:
        r = await self._command("REMOVE_CACHE_ENTRY", cache_id, key)
        return DeleteItemResponse(cacheId=r.get("cacheId"), key=r.get("key"), operationStatus=r.get("status"))

    async def cancel_item_removal(self, cache_id, key) -> CancelItemRemovalResponse:
        r = await self._command("CANCEL_CACHE_ITEM_REMOVAL", cache_id, key)
        return CancelItemRemovalResponse(cacheId=r.get("cacheId"), key=r.get("key"), operationStatus=r.get("status"))

    async def clear_cache(self, cache_id) -> ClearCacheResponse:
        r = await self._command("CLEAR_CACHE", cache_id)
        return ClearCacheResponse(cacheId=r.get("cacheId"), operationStatus=r.get("status"))

    async def delete_cache(self, cache_id) -> DeleteCacheResponse:
        r = await self._command("DELETE_CACHE", cache_id)
        return DeleteCacheResponse(cacheId=r.get("cacheId"), operationStatus=r.get("status"))

    async def get_cache_items(self, cache_id) -> GetCacheResponse:
        acc = await self._batched("GET_CACHE_ENTRIES", cache_id, "entries")
        items = [CacheItem(key=k, value=v) for k, v in acc["items"].items()]
        return GetCacheResponse(cacheId=acc.get("cacheId"), operationStatus=acc.get("status"), items=items)

    async def get_stats(self) -> list:
        acc = await self._batched("GET_CACHE_STATS", None, "stats")
        return [StatEntry(**s) for s in acc["stats"]]

    # ------------------------------------------------------------------ counter commands

    async def create_counter_cache(self, cache_id) -> CreateResponse:
        r = await self._command("CREATE_COUNTER_CACHE", cache_id)
        return CreateResponse(cacheId=r.get("cacheId"), operationStatus=r.get("status"))

    async def put_counter(self, cache_id, key, value) -> PutItemResponse:
        r = await self._command("ADD_COUNTER_ENTRY", cache_id, key, counter_value=value)
        return PutItemResponse(cacheId=r.get("cacheId"), key=r.get("key"), operationStatus=r.get("status"))

    async def put_timed_counter(self, cache_id, key, value, ttl) -> PutItemResponse:
        r = await self._command("ADD_COUNTER_ENTRY", cache_id, key, ttl=ttl, counter_value=value)
        return PutItemResponse(cacheId=r.get("cacheId"), key=r.get("key"), operationStatus=r.get("status"))

    async def get_counter(self, cache_id, key) -> CounterResponse:
        r = await self._command("GET_COUNTER_ENTRY", cache_id, key)
        return self._counter_response(r)

    async def delete_counter(self, cache_id, key) -> DeleteItemResponse:
        r = await self._command("REMOVE_COUNTER_ENTRY", cache_id, key)
        return DeleteItemResponse(cacheId=r.get("cacheId"), key=r.get("key"), operationStatus=r.get("status"))

    async def cancel_counter_item_removal(self, cache_id, key) -> CancelItemRemovalResponse:
        r = await self._command("CANCEL_COUNTER_ITEM_REMOVAL", cache_id, key)
        return CancelItemRemovalResponse(cacheId=r.get("cacheId"), key=r.get("key"), operationStatus=r.get("status"))

    async def clear_counter_cache(self, cache_id) -> ClearCacheResponse:
        r = await self._command("CLEAR_COUNTER_CACHE", cache_id)
        return ClearCacheResponse(cacheId=r.get("cacheId"), operationStatus=r.get("status"))

    async def delete_counter_cache(self, cache_id) -> DeleteCacheResponse:
        r = await self._command("DELETE_COUNTER_CACHE", cache_id)
        return DeleteCacheResponse(cacheId=r.get("cacheId"), operationStatus=r.get("status"))

    async def increment_counter(self, cache_id, key, amount) -> CounterResponse:
        r = await self._command("INCREMENT_COUNTER_ENTRY", cache_id, key, counter_value=amount)
        return self._counter_response(r)

    async def decrement_counter(self, cache_id, key, amount) -> CounterResponse:
        r = await self._command("DECREMENT_COUNTER_ENTRY", cache_id, key, counter_value=amount)
        return self._counter_response(r)

    async def set_counter(self, cache_id, key, value) -> CounterResponse:
        r = await self._command("SET_COUNTER_ENTRY", cache_id, key, counter_value=value)
        return self._counter_response(r)

    async def get_counter_items(self, cache_id) -> GetCountersResponse:
        acc = await self._batched("GET_COUNTER_ENTRIES", cache_id, "entries")
        items = [CounterItem(key=k, value=int(v)) for k, v in acc["items"].items()]
        return GetCountersResponse(cacheId=acc.get("cacheId"), operationStatus=acc.get("status"), items=items)

    async def get_counter_stats(self) -> list:
        acc = await self._batched("GET_COUNTER_STATS", None, "stats")
        return [StatEntry(**s) for s in acc["stats"]]

    @staticmethod
    def _counter_response(r) -> CounterResponse:
        value = r.get("value")
        return CounterResponse(
            cacheId=r.get("cacheId"), key=r.get("key"),
            value=(int(value) if value not in (None, "") else 0),
            operationStatus=r.get("status"),
        )

    # ------------------------------------------------------------------ subscriptions

    async def subscribe(self, cache_id, on_message, send_snapshot: bool = False, key=None, mode=None) -> BidiSubscription:
        return await self._subscribe(cache_id, on_message, counters=False, send_snapshot=send_snapshot, key=key, mode=mode)

    async def subscribe_counter(self, cache_id, on_message, send_snapshot: bool = False, key=None) -> BidiSubscription:
        return await self._subscribe(cache_id, on_message, counters=True, send_snapshot=send_snapshot, key=key, mode=None)

    async def _subscribe(self, cache_id, on_message, counters, send_snapshot, key, mode) -> BidiSubscription:
        cid = str(uuid.uuid4())
        selector = {"cacheId": cache_id}
        if key is not None:
            selector["key"] = key
        if mode is not None:
            selector["mode"] = mode.value if hasattr(mode, "value") else str(mode).upper()
        self._subs.setdefault(cache_id, []).append({"cid": cid, "on_message": on_message, "counters": counters})
        ack = asyncio.get_event_loop().create_future()
        self._sub_acks[cid] = ack
        await self._send({
            "type": "subscribe", "correlationId": cid, "counters": counters,
            "sendSnapshot": send_snapshot, "caches": [selector],
        })
        try:
            await asyncio.wait_for(ack, self.request_timeout)
        except asyncio.TimeoutError:
            # Proceed without a confirmed ack; updates may still arrive.
            self._sub_acks.pop(cid, None)
        return BidiSubscription(self, cid, cache_id, counters)

    async def _unsubscribe(self, correlation_id, cache_id, counters):
        subs = self._subs.get(cache_id)
        if subs is not None:
            remaining = [s for s in subs if s["cid"] != correlation_id]
            if remaining:
                self._subs[cache_id] = remaining
            else:
                self._subs.pop(cache_id, None)
        await self._send({
            "type": "unsubscribe", "correlationId": str(uuid.uuid4()),
            "counters": counters, "cacheId": cache_id,
        })
