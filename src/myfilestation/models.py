from dataclasses import dataclass
from enum import Enum
from typing import Optional
import uuid
import time


class ItemType(str, Enum):
    FILE = "file"
    TEXT_TEMP = "text_temp"
    IMAGE_TEMP = "image_temp"


@dataclass
class StationItem:
    # English comments as requested
    id: str
    item_type: ItemType
    path: str
    display_name: str
    is_pinned: bool = False
    thumbnail_path: Optional[str] = None
    added_at: float = time.time()

    @staticmethod
    def new(item_type: ItemType, path: str, display_name: str, thumbnail_path: Optional[str] = None) -> "StationItem":
        return StationItem(
            id=str(uuid.uuid4()),
            item_type=item_type,
            path=path,
            display_name=display_name,
            thumbnail_path=thumbnail_path,
        )
