from dataclasses import dataclass
from typing import Optional

@dataclass
class CreateResponse:
    cacheId: str
    operationStatus: str

@dataclass
class PutItemRequest:
    key: str
    value: str

@dataclass
class PutTimedItemRequest:
    key: str
    value: str
    ttl: int

@dataclass
class PutItemResponse:
    cacheId: str
    key: str
    operationStatus: str
    status: Optional[str] = "OK"

@dataclass
class GetItemResponse:
    cacheId: str
    key: str
    value: str
    operationStatus: str

@dataclass
class DeleteItemResponse:
    cacheId: str
    key: str
    operationStatus: str

@dataclass
class DeleteCacheResponse:
    cacheId: str
    operationStatus: str

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

@dataclass
class CacheItem:
    key: str
    value: str

@dataclass
class GetCacheResponse:
    cacheId: str
    operationStatus: str
    items: list[CacheItem]

    @classmethod
    def from_dict(cls, data: dict):
        items = [CacheItem(**item) for item in data.get('items', [])]
        return cls(
            cacheId=data.get('cacheId'),
            operationStatus=data.get('operationStatus'),
            items=items
        )

@dataclass
class ClearCacheResponse:
    cacheId: str
    operationStatus: str

from enum import Enum

class BulkOperationType(str, Enum):
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
    operationType: BulkOperationType
    requestId: str
    cacheId: str
    key: Optional[str] = None
    value: Optional[str] = None
    ttl: Optional[int] = None
    counterValue: Optional[int] = None

@dataclass
class BulkCacheOpsRequest:
    requestId: str
    operations: list[CacheOperationRequest]

@dataclass
class CacheOperationResponse:
    requestId: str
    status: str
    cacheId: str
    key: Optional[str] = None
    value: Optional[str] = None

@dataclass
class BulkCacheOpsResponse:
    requestId: str
    operationResponses: list[CacheOperationResponse]

    @classmethod
    def from_dict(cls, data: dict):
        ops = [CacheOperationResponse(**op) for op in data.get('operationResponses', [])]
        return cls(
            requestId=data.get('requestId'),
            operationResponses=ops
        )
