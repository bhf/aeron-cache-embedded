/**
 * Bidirectional WebSocket transport for Aeron Cache.
 *
 * A single WebSocket connection to `/api/ws/v1/bidi` carries the full cache + counter command
 * surface plus dynamic subscribe/unsubscribe, multiplexed by a client-minted `correlationId` that
 * the server echoes on every correlated frame. This is the JSON/WebSocket analogue of the Aeron
 * gateway transport, and an alternative to the HTTP+WS {@link AeronCacheClient}.
 *
 * The API is async (the connection is persistent); method names and response models mirror the
 * HTTP client. Uses the same WebSocket implementation as the HTTP client's subscriptions — the
 * runtime global `WebSocket` (available in browsers and Node.js >= 22).
 */

import {
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
    TimerInfo,
    BulkCacheOpsRequest,
    BulkCacheOpsResponse,
    CacheOperationResponse
} from './models';

/** WsOp values accepted by the bidi command frame. */
export type WsOp =
    | 'CREATE_CACHE'
    | 'ADD_CACHE_ENTRY'
    | 'PATCH_CACHE_ENTRY'
    | 'GET_CACHE_ENTRY'
    | 'CLEAR_CACHE'
    | 'DELETE_CACHE'
    | 'REMOVE_CACHE_ENTRY'
    | 'CANCEL_CACHE_ITEM_REMOVAL'
    | 'GET_CACHE_ENTRIES'
    | 'GET_CACHE_STATS'
    | 'GET_TIMERS'
    | 'CREATE_COUNTER_CACHE'
    | 'ADD_COUNTER_ENTRY'
    | 'GET_COUNTER_ENTRY'
    | 'CLEAR_COUNTER_CACHE'
    | 'DELETE_COUNTER_CACHE'
    | 'REMOVE_COUNTER_ENTRY'
    | 'CANCEL_COUNTER_ITEM_REMOVAL'
    | 'GET_COUNTER_ENTRIES'
    | 'GET_COUNTER_STATS'
    | 'INCREMENT_COUNTER_ENTRY'
    | 'DECREMENT_COUNTER_ENTRY'
    | 'SET_COUNTER_ENTRY';

/** Cache subscription mode. */
export type SubscriptionMode = 'full' | 'patch';

/** An error frame correlated to a request. */
export class BidiError extends Error {
    readonly status?: string;
    constructor(status: string | undefined, message: string | undefined) {
        super(`${status}: ${message}`);
        this.name = 'BidiError';
        this.status = status;
    }
}

/** Handle for an active subscription; `close()` (alias `unsubscribe()`) to stop it. */
export interface BidiSubscription {
    readonly correlationId: string;
    readonly cacheId: string;
    readonly counters: boolean;
    close(): Promise<void>;
    unsubscribe(): Promise<void>;
}

interface Pending {
    resolve: (msg: any) => void;
    reject: (err: any) => void;
    timer: any;
}

interface BatchAcc {
    items: Record<string, any>;
    stats: any[];
    timers: any[];
    cacheId: string | null;
    status: string | null;
    resolve: (acc: BatchAcc) => void;
    reject: (err: any) => void;
    timer: any;
}

interface BulkAcc {
    operations: any[];
    resolve: (acc: BulkAcc) => void;
    reject: (err: any) => void;
    timer: any;
}

interface Listener {
    cid: string;
    onMessage: (ev: any) => void;
    counters: boolean;
}

interface SubscribeOpts {
    sendSnapshot?: boolean;
    key?: string;
    mode?: SubscriptionMode;
}

function uuid(): string {
    // crypto.randomUUID is available in browsers and Node >= 19.
    const c: any = (globalThis as any).crypto;
    if (c && typeof c.randomUUID === 'function') {
        return c.randomUUID();
    }
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (ch) => {
        const r = (Math.random() * 16) | 0;
        const v = ch === 'x' ? r : (r & 0x3) | 0x8;
        return v.toString(16);
    });
}

export class AeronBidiClient {
    private readonly wsUrl: string;
    private readonly requestTimeout: number;
    private ws: WebSocket | null = null;
    private connectPromise: Promise<void> | null = null;

    // correlationId -> single-response command
    private readonly pending = new Map<string, Pending>();
    // correlationId -> accumulating entries/stats batch
    private readonly entries = new Map<string, BatchAcc>();
    private readonly statsBatches = new Map<string, BatchAcc>();
    // correlationId -> accumulating timers batch
    private readonly timersBatches = new Map<string, BatchAcc>();
    // correlationId -> accumulating bulk-operations batch
    private readonly bulkBatches = new Map<string, BulkAcc>();
    // correlationId -> subscribe ack resolver
    private readonly subAcks = new Map<string, { resolve: (ids: string[]) => void; reject: (err: any) => void; timer: any }>();
    // cacheId -> listeners. Stream updates are routed by cacheId because the server stamps
    // streamUpdate.correlationId with the *causing command's* id (the update's requestId),
    // not the subscription's id.
    private readonly subs = new Map<string, Listener[]>();

    constructor(wsUrl: string, requestTimeout = 10000) {
        this.wsUrl = wsUrl.replace(/\/$/, '');
        this.requestTimeout = requestTimeout;
    }

    // ------------------------------------------------------------------ lifecycle

    async connect(): Promise<void> {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            return;
        }
        if (this.connectPromise) {
            return this.connectPromise;
        }
        this.connectPromise = new Promise<void>((resolve, reject) => {
            let settled = false;
            const ws = new WebSocket(`${this.wsUrl}/api/ws/v1/bidi`);
            this.ws = ws;
            ws.onopen = () => {
                settled = true;
                resolve();
            };
            ws.onmessage = (event: MessageEvent) => {
                let msg: any;
                try {
                    msg = JSON.parse(typeof event.data === 'string' ? event.data : String(event.data));
                } catch {
                    return;
                }
                try {
                    this.dispatch(msg);
                } catch {
                    // never let a handler error break the read loop
                }
            };
            ws.onerror = (err: any) => {
                if (!settled) {
                    settled = true;
                    this.connectPromise = null;
                    reject(new Error('AeronBidiClient failed to connect'));
                }
            };
            ws.onclose = () => {
                this.ws = null;
                this.connectPromise = null;
                this.failAll(new Error('AeronBidiClient connection closed'));
            };
        });
        return this.connectPromise;
    }

    async close(): Promise<void> {
        const ws = this.ws;
        this.ws = null;
        this.connectPromise = null;
        if (ws) {
            ws.onclose = null as any;
            ws.close();
        }
        this.failAll(new Error('AeronBidiClient closed'));
    }

    // ------------------------------------------------------------------ dispatch

    private dispatch(msg: any): void {
        const type = msg.type;
        const cid = msg.correlationId;
        switch (type) {
            case 'commandResponse': {
                const p = this.pending.get(cid);
                if (p) {
                    this.pending.delete(cid);
                    clearTimeout(p.timer);
                    p.resolve(msg);
                }
                break;
            }
            case 'entries': {
                const acc = this.entries.get(cid);
                if (acc) {
                    Object.assign(acc.items, msg.items || {});
                    acc.cacheId = msg.cacheId;
                    acc.status = msg.status;
                    if (msg.endOfBatch) {
                        this.entries.delete(cid);
                        clearTimeout(acc.timer);
                        acc.resolve(acc);
                    }
                }
                break;
            }
            case 'stats': {
                const acc = this.statsBatches.get(cid);
                if (acc) {
                    if (Array.isArray(msg.stats)) {
                        acc.stats.push(...msg.stats);
                    }
                    acc.status = msg.status;
                    if (msg.endOfBatch) {
                        this.statsBatches.delete(cid);
                        clearTimeout(acc.timer);
                        acc.resolve(acc);
                    }
                }
                break;
            }
            case 'timers': {
                const acc = this.timersBatches.get(cid);
                if (acc) {
                    if (Array.isArray(msg.timers)) {
                        acc.timers.push(...msg.timers);
                    }
                    acc.status = msg.status;
                    if (msg.endOfBatch) {
                        this.timersBatches.delete(cid);
                        clearTimeout(acc.timer);
                        acc.resolve(acc);
                    }
                }
                break;
            }
            case 'bulkResponse': {
                const acc = this.bulkBatches.get(cid);
                if (acc) {
                    if (Array.isArray(msg.operationResponses)) {
                        acc.operations.push(...msg.operationResponses);
                    }
                    if (msg.endOfBatch) {
                        this.bulkBatches.delete(cid);
                        clearTimeout(acc.timer);
                        acc.resolve(acc);
                    }
                }
                break;
            }
            case 'subscribed': {
                const a = this.subAcks.get(cid);
                if (a) {
                    this.subAcks.delete(cid);
                    clearTimeout(a.timer);
                    a.resolve(msg.cacheIds || []);
                }
                break;
            }
            case 'streamUpdate':
                this.dispatchStreamUpdate(cid, msg);
                break;
            case 'error':
                this.dispatchError(cid, msg);
                break;
            default:
                break;
        }
    }

    private dispatchStreamUpdate(cid: string, msg: any): void {
        const cacheId = msg.cacheId;
        const listeners = this.subs.get(cacheId);
        if (!listeners || listeners.length === 0) {
            return;
        }
        const value = msg.value;
        for (const sub of [...listeners]) {
            if (sub.counters) {
                const ev: CounterUpdateEvent = {
                    cacheId,
                    eventType: msg.eventType,
                    requestId: cid,
                    itemKey: msg.key,
                    itemValue: value == null ? undefined : Number(value)
                };
                sub.onMessage(ev);
            } else {
                const ev: CacheUpdateEvent = {
                    cacheId,
                    eventType: msg.eventType,
                    requestId: cid,
                    itemKey: msg.key,
                    itemValue: value == null ? undefined : String(value)
                };
                sub.onMessage(ev);
            }
        }
    }

    private dispatchError(cid: string | undefined, msg: any): void {
        const err = new BidiError(msg.status, msg.message);
        if (cid == null) {
            return;
        }
        const p = this.pending.get(cid);
        if (p) {
            this.pending.delete(cid);
            clearTimeout(p.timer);
            p.reject(err);
            return;
        }
        const e = this.entries.get(cid);
        if (e) {
            this.entries.delete(cid);
            clearTimeout(e.timer);
            e.reject(err);
            return;
        }
        const s = this.statsBatches.get(cid);
        if (s) {
            this.statsBatches.delete(cid);
            clearTimeout(s.timer);
            s.reject(err);
            return;
        }
        const t = this.timersBatches.get(cid);
        if (t) {
            this.timersBatches.delete(cid);
            clearTimeout(t.timer);
            t.reject(err);
            return;
        }
        const b = this.bulkBatches.get(cid);
        if (b) {
            this.bulkBatches.delete(cid);
            clearTimeout(b.timer);
            b.reject(err);
            return;
        }
        const a = this.subAcks.get(cid);
        if (a) {
            this.subAcks.delete(cid);
            clearTimeout(a.timer);
            a.reject(err);
        }
    }

    private failAll(err: any): void {
        for (const p of this.pending.values()) {
            clearTimeout(p.timer);
            p.reject(err);
        }
        this.pending.clear();
        for (const acc of this.entries.values()) {
            clearTimeout(acc.timer);
            acc.reject(err);
        }
        this.entries.clear();
        for (const acc of this.statsBatches.values()) {
            clearTimeout(acc.timer);
            acc.reject(err);
        }
        this.statsBatches.clear();
        for (const acc of this.timersBatches.values()) {
            clearTimeout(acc.timer);
            acc.reject(err);
        }
        this.timersBatches.clear();
        for (const acc of this.bulkBatches.values()) {
            clearTimeout(acc.timer);
            acc.reject(err);
        }
        this.bulkBatches.clear();
        for (const a of this.subAcks.values()) {
            clearTimeout(a.timer);
            a.reject(err);
        }
        this.subAcks.clear();
    }

    // ------------------------------------------------------------------ send helpers

    private async send(frame: any): Promise<void> {
        await this.connect();
        this.ws!.send(JSON.stringify(frame));
    }

    private async command(
        op: WsOp,
        cacheId: string | null = null,
        key: string | null = null,
        value: string | null = null,
        ttl = 0,
        counterValue = 0
    ): Promise<any> {
        const cid = uuid();
        const promise = new Promise<any>((resolve, reject) => {
            const timer = setTimeout(() => {
                this.pending.delete(cid);
                reject(new Error(`bidi command ${op} timed out`));
            }, this.requestTimeout);
            this.pending.set(cid, { resolve, reject, timer });
        });
        await this.send({ type: 'command', correlationId: cid, op, cacheId, key, value, ttl, counterValue });
        return promise;
    }

    private async batched(op: WsOp, cacheId: string | null, kind: 'entries' | 'stats' | 'timers'): Promise<BatchAcc> {
        const cid = uuid();
        const map = kind === 'entries' ? this.entries : kind === 'stats' ? this.statsBatches : this.timersBatches;
        const promise = new Promise<BatchAcc>((resolve, reject) => {
            const timer = setTimeout(() => {
                map.delete(cid);
                reject(new Error(`bidi command ${op} timed out`));
            }, this.requestTimeout);
            const acc: BatchAcc = { items: {}, stats: [], timers: [], cacheId, status: null, resolve, reject, timer };
            map.set(cid, acc);
        });
        await this.send({ type: 'command', correlationId: cid, op, cacheId, key: null, value: null, ttl: 0, counterValue: 0 });
        return promise;
    }

    private async bulk(operations: any[]): Promise<BulkAcc> {
        const cid = uuid();
        const promise = new Promise<BulkAcc>((resolve, reject) => {
            const timer = setTimeout(() => {
                this.bulkBatches.delete(cid);
                reject(new Error('bidi bulk operation timed out'));
            }, this.requestTimeout);
            const acc: BulkAcc = { operations: [], resolve, reject, timer };
            this.bulkBatches.set(cid, acc);
        });
        await this.send({ type: 'bulk', correlationId: cid, operations });
        return promise;
    }

    // ------------------------------------------------------------------ cache commands

    async createCache(cacheId: string): Promise<CreateResponse> {
        const r = await this.command('CREATE_CACHE', cacheId);
        return { cacheId: r.cacheId, operationStatus: r.status };
    }

    async putItem(cacheId: string, key: string, value: string): Promise<PutItemResponse> {
        return this.putTimedItem(cacheId, key, value, 0);
    }

    async putTimedItem(cacheId: string, key: string, value: string, ttl: number): Promise<PutItemResponse> {
        const r = await this.command('ADD_CACHE_ENTRY', cacheId, key, value, ttl);
        return { cacheId: r.cacheId, key: r.key, operationStatus: r.status };
    }

    async patchItem(cacheId: string, key: string, value: string): Promise<PatchItemResponse> {
        const r = await this.command('PATCH_CACHE_ENTRY', cacheId, key, value);
        return { cacheId: r.cacheId, key: r.key, operationStatus: r.status };
    }

    async getItem(cacheId: string, key: string): Promise<GetItemResponse> {
        const r = await this.command('GET_CACHE_ENTRY', cacheId, key);
        return { cacheId: r.cacheId, key: r.key, value: r.value, operationStatus: r.status };
    }

    async deleteItem(cacheId: string, key: string): Promise<DeleteItemResponse> {
        const r = await this.command('REMOVE_CACHE_ENTRY', cacheId, key);
        return { cacheId: r.cacheId, key: r.key, operationStatus: r.status };
    }

    async cancelItemRemoval(cacheId: string, key: string): Promise<CancelItemRemovalResponse> {
        const r = await this.command('CANCEL_CACHE_ITEM_REMOVAL', cacheId, key);
        return { cacheId: r.cacheId, key: r.key, operationStatus: r.status };
    }

    async clearCache(cacheId: string): Promise<ClearCacheResponse> {
        const r = await this.command('CLEAR_CACHE', cacheId);
        return { cacheId: r.cacheId, operationStatus: r.status };
    }

    async deleteCache(cacheId: string): Promise<DeleteCacheResponse> {
        const r = await this.command('DELETE_CACHE', cacheId);
        return { cacheId: r.cacheId, operationStatus: r.status };
    }

    async getCacheItems(cacheId: string): Promise<GetCacheResponse> {
        const acc = await this.batched('GET_CACHE_ENTRIES', cacheId, 'entries');
        const items: CacheItem[] = Object.entries(acc.items).map(([key, value]) => ({ key, value: String(value) }));
        return { cacheId: acc.cacheId ?? cacheId, operationStatus: acc.status ?? '', items };
    }

    async getStats(): Promise<StatEntry[]> {
        const acc = await this.batched('GET_CACHE_STATS', null, 'stats');
        return acc.stats.map((s) => ({
            cacheId: s.cacheId,
            addedCount: s.addedCount,
            removedCount: s.removedCount,
            clearedCount: s.clearedCount,
            size: s.size
        }));
    }

    // ------------------------------------------------------------------ timers

    /**
     * Get all pending TTL removal timers across both caches and counter caches. The server streams
     * one or more `timers` frames, accumulated until the end-of-batch frame; the returned list
     * carries every timer.
     */
    async getTimers(): Promise<TimerInfo[]> {
        const acc = await this.batched('GET_TIMERS', null, 'timers');
        return acc.timers.map((t) => ({
            timerType: t.timerType,
            cacheId: t.cacheId,
            key: t.key,
            deadline: Number(t.deadline)
        }));
    }

    // ------------------------------------------------------------------ bulk operations

    /**
     * Apply a batch of cache/counter operations in one `bulk` frame, mirroring the HTTP client's
     * {@link AeronCacheClient.bulkOps}. A batch may freely mix regular-cache and counter operations.
     * The server streams one or more `bulkResponse` frames, accumulated until the end-of-batch frame.
     */
    async bulkOps(request: BulkCacheOpsRequest): Promise<BulkCacheOpsResponse> {
        const operations = request.operations ?? [];
        const acc = await this.bulk(operations);
        const operationResponses: CacheOperationResponse[] = acc.operations.map((o) => ({
            requestId: o.requestId,
            status: o.status,
            cacheId: o.cacheId,
            key: o.key,
            value: o.value
        }));
        return { requestId: request.requestId, operationResponses };
    }

    // ------------------------------------------------------------------ counter commands

    async createCounterCache(cacheId: string): Promise<CreateResponse> {
        const r = await this.command('CREATE_COUNTER_CACHE', cacheId);
        return { cacheId: r.cacheId, operationStatus: r.status };
    }

    async putCounter(cacheId: string, key: string, value: number): Promise<PutItemResponse> {
        const r = await this.command('ADD_COUNTER_ENTRY', cacheId, key, null, 0, value);
        return { cacheId: r.cacheId, key: r.key, operationStatus: r.status };
    }

    async putTimedCounter(cacheId: string, key: string, value: number, ttl: number): Promise<PutItemResponse> {
        const r = await this.command('ADD_COUNTER_ENTRY', cacheId, key, null, ttl, value);
        return { cacheId: r.cacheId, key: r.key, operationStatus: r.status };
    }

    async getCounter(cacheId: string, key: string): Promise<CounterResponse> {
        const r = await this.command('GET_COUNTER_ENTRY', cacheId, key);
        return this.counterResponse(r);
    }

    async deleteCounter(cacheId: string, key: string): Promise<DeleteItemResponse> {
        const r = await this.command('REMOVE_COUNTER_ENTRY', cacheId, key);
        return { cacheId: r.cacheId, key: r.key, operationStatus: r.status };
    }

    async cancelCounterItemRemoval(cacheId: string, key: string): Promise<CancelItemRemovalResponse> {
        const r = await this.command('CANCEL_COUNTER_ITEM_REMOVAL', cacheId, key);
        return { cacheId: r.cacheId, key: r.key, operationStatus: r.status };
    }

    async clearCounterCache(cacheId: string): Promise<ClearCacheResponse> {
        const r = await this.command('CLEAR_COUNTER_CACHE', cacheId);
        return { cacheId: r.cacheId, operationStatus: r.status };
    }

    async deleteCounterCache(cacheId: string): Promise<DeleteCacheResponse> {
        const r = await this.command('DELETE_COUNTER_CACHE', cacheId);
        return { cacheId: r.cacheId, operationStatus: r.status };
    }

    async incrementCounter(cacheId: string, key: string, amount: number): Promise<CounterResponse> {
        const r = await this.command('INCREMENT_COUNTER_ENTRY', cacheId, key, null, 0, amount);
        return this.counterResponse(r);
    }

    async decrementCounter(cacheId: string, key: string, amount: number): Promise<CounterResponse> {
        const r = await this.command('DECREMENT_COUNTER_ENTRY', cacheId, key, null, 0, amount);
        return this.counterResponse(r);
    }

    async setCounter(cacheId: string, key: string, value: number): Promise<CounterResponse> {
        const r = await this.command('SET_COUNTER_ENTRY', cacheId, key, null, 0, value);
        return this.counterResponse(r);
    }

    async getCounterItems(cacheId: string): Promise<GetCountersResponse> {
        const acc = await this.batched('GET_COUNTER_ENTRIES', cacheId, 'entries');
        const items: CounterItem[] = Object.entries(acc.items).map(([key, value]) => ({ key, value: Number(value) }));
        return { cacheId: acc.cacheId ?? cacheId, operationStatus: acc.status ?? '', items };
    }

    async getCounterStats(): Promise<StatEntry[]> {
        const acc = await this.batched('GET_COUNTER_STATS', null, 'stats');
        return acc.stats.map((s) => ({
            cacheId: s.cacheId,
            addedCount: s.addedCount,
            removedCount: s.removedCount,
            clearedCount: s.clearedCount,
            size: s.size
        }));
    }

    private counterResponse(r: any): CounterResponse {
        const value = r.value;
        return {
            cacheId: r.cacheId,
            key: r.key,
            value: value == null || value === '' ? 0 : Number(value),
            operationStatus: r.status
        };
    }

    // ------------------------------------------------------------------ subscriptions

    async subscribe(
        cacheId: string,
        onEvent: (ev: CacheUpdateEvent) => void,
        opts: SubscribeOpts = {}
    ): Promise<BidiSubscription> {
        return this.doSubscribe(cacheId, onEvent, false, opts);
    }

    async subscribeCounter(
        cacheId: string,
        onEvent: (ev: CounterUpdateEvent) => void,
        opts: SubscribeOpts = {}
    ): Promise<BidiSubscription> {
        return this.doSubscribe(cacheId, onEvent, true, { sendSnapshot: opts.sendSnapshot, key: opts.key });
    }

    private async doSubscribe(
        cacheId: string,
        onEvent: (ev: any) => void,
        counters: boolean,
        opts: SubscribeOpts
    ): Promise<BidiSubscription> {
        const cid = uuid();
        const selector: any = { cacheId };
        if (opts.key != null) {
            selector.key = opts.key;
        }
        if (opts.mode != null) {
            selector.mode = String(opts.mode).toUpperCase();
        }
        const listeners = this.subs.get(cacheId) ?? [];
        listeners.push({ cid, onMessage: onEvent, counters });
        this.subs.set(cacheId, listeners);

        const ackPromise = new Promise<string[]>((resolve, reject) => {
            const timer = setTimeout(() => {
                // Proceed without a confirmed ack; updates may still arrive.
                this.subAcks.delete(cid);
                resolve([]);
            }, this.requestTimeout);
            this.subAcks.set(cid, { resolve, reject, timer });
        });
        await this.send({
            type: 'subscribe',
            correlationId: cid,
            counters,
            sendSnapshot: opts.sendSnapshot ?? false,
            caches: [selector]
        });
        try {
            await ackPromise;
        } catch {
            // best-effort ack; ignore rejection and keep the subscription active
        }

        const unsubscribe = () => this.unsubscribe(cid, cacheId, counters);
        return {
            correlationId: cid,
            cacheId,
            counters,
            close: unsubscribe,
            unsubscribe
        };
    }

    private async unsubscribe(correlationId: string, cacheId: string, counters: boolean): Promise<void> {
        const listeners = this.subs.get(cacheId);
        if (listeners) {
            const remaining = listeners.filter((s) => s.cid !== correlationId);
            if (remaining.length > 0) {
                this.subs.set(cacheId, remaining);
            } else {
                this.subs.delete(cacheId);
            }
        }
        await this.send({ type: 'unsubscribe', correlationId: uuid(), counters, cacheId });
    }
}
