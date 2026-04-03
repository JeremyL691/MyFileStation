from dataclasses import dataclass, field
from enum import Enum
from typing import Optional
import uuid
import time
import os


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
    added_at: float = field(default_factory=time.time)

    @staticmethod
    def new(item_type: ItemType, path: str, display_name: str, thumbnail_path: Optional[str] = None) -> "StationItem":
        return StationItem(
            id=str(uuid.uuid4()),
            item_type=item_type,
            path=path,
            display_name=display_name,
            thumbnail_path=thumbnail_path,
        )

    @property
    def is_temp(self) -> bool:
        return self.item_type in {ItemType.TEXT_TEMP, ItemType.IMAGE_TEMP}

    @property
    def exists(self) -> bool:
        return os.path.exists(self.path)

    @property
    def kind_label(self) -> str:
        if self.item_type == ItemType.TEXT_TEMP:
            return "TEXT"
        if self.item_type == ItemType.IMAGE_TEMP:
            return "IMAGE"
        return "FILE"
