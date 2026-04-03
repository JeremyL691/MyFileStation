from __future__ import annotations

import os
from typing import Callable, Iterable, List, Sequence, Tuple

from .models import ItemType, StationItem
from .settings import AppSettings
from .utils import create_temp_image_file_from_qimage, create_temp_text_file, delete_file_if_exists


class ShelfController:
    def __init__(
        self,
        settings: AppSettings,
        temp_text_factory: Callable[[str], str] = create_temp_text_file,
        temp_image_factory: Callable[[object], str] = create_temp_image_file_from_qimage,
    ) -> None:
        self.settings = settings
        self._temp_text_factory = temp_text_factory
        self._temp_image_factory = temp_image_factory
        self.items: List[StationItem] = []

    def get_items(self) -> List[StationItem]:
        return list(self.items)

    def find_item(self, item_id: str) -> StationItem | None:
        for item in self.items:
            if item.id == item_id:
                return item
        return None

    def add_file(self, path: str) -> StationItem:
        if not os.path.exists(path):
            raise FileNotFoundError(path)
        item = StationItem.new(
            ItemType.FILE,
            path,
            os.path.basename(path),
            path if _is_image_file(path) else None,
        )
        self.items.append(item)
        return item

    def add_temp_text(self, text: str) -> StationItem:
        path = self._temp_text_factory(text)
        item = StationItem.new(ItemType.TEXT_TEMP, path, os.path.basename(path), None)
        self.items.append(item)
        return item

    def add_temp_image(self, image: object) -> StationItem:
        path = self._temp_image_factory(image)
        item = StationItem.new(ItemType.IMAGE_TEMP, path, os.path.basename(path), path)
        self.items.append(item)
        return item

    def set_pinned(self, item_id: str, pinned: bool) -> StationItem:
        item = self._require_item(item_id)
        item.is_pinned = pinned
        return item

    def remove_item(self, item_id: str, force: bool = False) -> StationItem | None:
        item = self.find_item(item_id)
        if item is None:
            return None
        if item.is_pinned and not force:
            return None

        self.items = [current for current in self.items if current.id != item_id]
        if item.is_temp:
            delete_file_if_exists(item.path)
        return item

    def clear_unlocked(self) -> List[StationItem]:
        removable_ids = [item.id for item in self.items if not item.is_pinned]
        removed = []
        for item_id in removable_ids:
            removed_item = self.remove_item(item_id, force=True)
            if removed_item is not None:
                removed.append(removed_item)
        return removed

    def handle_drag_out_result(self, item_ids: Sequence[str], success: bool) -> List[StationItem]:
        if not success or not self.settings.remove_after_drag_out:
            return []

        removed = []
        for item_id in item_ids:
            item = self.find_item(item_id)
            if item is None or item.is_pinned:
                continue
            removed_item = self.remove_item(item_id, force=True)
            if removed_item is not None:
                removed.append(removed_item)
        return removed

    def existing_paths_for_ids(self, item_ids: Iterable[str]) -> Tuple[List[str], List[StationItem]]:
        paths: List[str] = []
        missing: List[StationItem] = []

        for item_id in item_ids:
            item = self.find_item(item_id)
            if item is None:
                continue
            if os.path.exists(item.path):
                paths.append(item.path)
            else:
                missing.append(item)

        return paths, missing

    def cleanup_on_exit(self) -> List[str]:
        if not self.settings.cleanup_temp_on_exit:
            return []

        cleaned_paths = []
        for item in self.items:
            if item.is_temp and not item.is_pinned:
                delete_file_if_exists(item.path)
                cleaned_paths.append(item.path)
        return cleaned_paths

    def remove_missing_item(self, item_id: str) -> StationItem | None:
        item = self.find_item(item_id)
        if item is None:
            return None
        self.items = [current for current in self.items if current.id != item_id]
        return item

    def _require_item(self, item_id: str) -> StationItem:
        item = self.find_item(item_id)
        if item is None:
            raise KeyError(item_id)
        return item


def _is_image_file(path: str) -> bool:
    ext = os.path.splitext(path)[1].lower()
    return ext in [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"]
