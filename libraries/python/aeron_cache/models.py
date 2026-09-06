from dataclasses import dataclass, field
from typing import Optional, List

@dataclass
class CreateResponse:
    cacheId: str
    operationStatus: Optional[str] = None

@dataclass
class PutItemResponse:
    cacheId: str
    key: str
    status: Optional[str] = "OK"
    operationStatus: Optional[str] = None

@dataclass
class GetItemResponse:
    cacheId: str
    key: str
    value: str
    operationStatus: Optional[str] = None

@dataclass
class DeleteItemResponse:
    cacheId: str
    key: str
    operationStatus: Optional[str] = None

@dataclass
class DeleteCacheResponse:
    cacheId: str
    operationStatus: Optional[str] = None

@dataclass
class ErrorResponse:
    errorMsg: str
    helpMsg: str

@dataclass
class CacheUpdateEvent:
    cacheId: str
    eventType: str
    requestId: str
    itemKey: Optional[str] = None
    itemValue: Optional[str] = None

    @classmethod
    def from_dict(cls, data: dict):
        return cls(
            cacheId=data.get('cacheId'),
            eventType=data.get('eventType'),
            requestId=data.get('requestId'),
            itemKey=data.get('itemKey'),
            itemValue=data.get('itemValue')
        )

# --- Counters ---

@dataclass
class CounterResponse:
    cacheId: str
    key: str
    value: int = 0
    operationStatus: Optional[str] = None

@dataclass
class CounterUpdateEvent:
    cacheId: str
    eventType: str
    requestId: str
    itemKey: Optional[str] = None
    itemValue: Optional[int] = None

    @classmethod
    def from_dict(cls, data: dict):
        return cls(
            cacheId=data.get('cacheId'),
            eventType=data.get('eventType'),
            requestId=data.get('requestId'),
            itemKey=data.get('itemKey'),
            itemValue=data.get('itemValue')
        )

# --- Bulk operations ---

class BulkOperationType:
    NONE = "NONE"
    CREATE_CACHE = "CREATE_CACHE"
    ADD_ITEM = "ADD_ITEM"
    REMOVE_ITEM = "REMOVE_ITEM"
    CLEAR_CACHE = "CLEAR_CACHE"
    GET_ITEM = "GET_ITEM"
    DELETE_CACHE = "DELETE_CACHE"
    CREATE_COUNTER_CACHE = "CREATE_COUNTER_CACHE"
    ADD_COUNTER = "ADD_COUNTER"
    REMOVE_COUNTER = "REMOVE_COUNTER"
    CLEAR_COUNTER_CACHE = "CLEAR_COUNTER_CACHE"
    GET_COUNTER = "GET_COUNTER"
    DELETE_COUNTER_CACHE = "DELETE_COUNTER_CACHE"
    INCREMENT_COUNTER = "INCREMENT_COUNTER"
    DECREMENT_COUNTER = "DECREMENT_COUNTER"
    SET_COUNTER = "SET_COUNTER"

@dataclass
class CacheOperationRequest:
    operationType: str
    cacheId: Optional[str] = None
    key: Optional[str] = None
    value: Optional[str] = None
    ttl: Optional[int] = None
    counterValue: Optional[int] = None
    requestId: Optional[str] = None

    def to_dict(self) -> dict:
        # Only include fields that are set, so the payload stays clean.
        return {k: v for k, v in self.__dict__.items() if v is not None}

@dataclass
class BulkCacheOpsRequest:
    operations: List[CacheOperationRequest] = field(default_factory=list)
    requestId: Optional[str] = None

    def to_dict(self) -> dict:
        payload = {"operations": [op.to_dict() for op in self.operations]}
        if self.requestId is not None:
            payload["requestId"] = self.requestId
        return payload

@dataclass
class CacheOperationResponse:
    requestId: Optional[str] = None
    status: Optional[str] = None
    cacheId: Optional[str] = None
    key: Optional[str] = None
    value: Optional[str] = None

    @classmethod
    def from_dict(cls, data: dict):
        return cls(
            requestId=data.get('requestId'),
            status=data.get('status'),
            cacheId=data.get('cacheId'),
            key=data.get('key'),
            value=data.get('value')
        )

@dataclass
class BulkCacheOpsResponse:
    requestId: Optional[str] = None
    operationResponses: List[CacheOperationResponse] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: dict):
        return cls(
            requestId=data.get('requestId'),
            operationResponses=[
                CacheOperationResponse.from_dict(r) for r in (data.get('operationResponses') or [])
            ]
        )
