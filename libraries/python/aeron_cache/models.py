from dataclasses import dataclass
from typing import Optional

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
