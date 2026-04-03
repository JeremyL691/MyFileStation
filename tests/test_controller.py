from pathlib import Path

from PySide6 import QtGui

from myfilestation.controller import ShelfController
from myfilestation.models import ItemType, StationItem
from myfilestation.settings import AppSettings


def test_station_item_metadata_helpers(tmp_path):
    file_path = tmp_path / "example.txt"
    file_path.write_text("hello", encoding="utf-8")

    item = StationItem.new(ItemType.FILE, str(file_path), "example.txt")

    assert item.is_temp is False
    assert item.exists is True
    assert item.kind_label == "FILE"


def test_controller_removes_unpinned_temp_file(tmp_path, monkeypatch):
    monkeypatch.setenv("TEMP", str(tmp_path))
    controller = ShelfController(AppSettings())

    item = controller.add_temp_text("hello")
    temp_path = Path(item.path)

    assert temp_path.exists()

    controller.remove_item(item.id)

    assert controller.find_item(item.id) is None
    assert not temp_path.exists()


def test_controller_keeps_pinned_temp_until_forced_remove(tmp_path, monkeypatch):
    monkeypatch.setenv("TEMP", str(tmp_path))
    controller = ShelfController(AppSettings())

    item = controller.add_temp_text("hello")
    controller.set_pinned(item.id, True)

    assert controller.remove_item(item.id) is None
    assert Path(item.path).exists()

    controller.remove_item(item.id, force=True)

    assert not Path(item.path).exists()


def test_controller_drag_out_respects_pin_and_setting(tmp_path):
    settings = AppSettings(remove_after_drag_out=True)
    controller = ShelfController(settings)

    file_a = tmp_path / "a.txt"
    file_a.write_text("a", encoding="utf-8")
    file_b = tmp_path / "b.txt"
    file_b.write_text("b", encoding="utf-8")

    item_a = controller.add_file(str(file_a))
    item_b = controller.add_file(str(file_b))
    controller.set_pinned(item_b.id, True)

    removed = controller.handle_drag_out_result([item_a.id, item_b.id], success=True)

    assert [item.id for item in removed] == [item_a.id]
    assert controller.find_item(item_a.id) is None
    assert controller.find_item(item_b.id) is not None


def test_controller_cleanup_on_exit_only_cleans_unpinned_temp(tmp_path, monkeypatch):
    monkeypatch.setenv("TEMP", str(tmp_path))
    controller = ShelfController(AppSettings(cleanup_temp_on_exit=True))

    unpinned = controller.add_temp_text("temp")
    pinned = controller.add_temp_text("keep")
    controller.set_pinned(pinned.id, True)

    cleaned = controller.cleanup_on_exit()

    assert unpinned.path in cleaned
    assert not Path(unpinned.path).exists()
    assert Path(pinned.path).exists()


def test_controller_existing_paths_reports_missing(tmp_path):
    controller = ShelfController(AppSettings())

    file_path = tmp_path / "exists.txt"
    file_path.write_text("x", encoding="utf-8")
    item_existing = controller.add_file(str(file_path))

    missing_path = tmp_path / "missing.txt"
    item_missing = StationItem.new(ItemType.FILE, str(missing_path), "missing.txt")
    controller.items.append(item_missing)

    paths, missing = controller.existing_paths_for_ids([item_existing.id, item_missing.id])

    assert paths == [str(file_path)]
    assert [item.id for item in missing] == [item_missing.id]


def test_controller_add_temp_image_creates_png(tmp_path, monkeypatch):
    monkeypatch.setenv("TEMP", str(tmp_path))
    controller = ShelfController(AppSettings())
    image = QtGui.QImage(4, 4, QtGui.QImage.Format_ARGB32)
    image.fill(QtGui.QColor("red"))

    item = controller.add_temp_image(image)

    assert item.item_type == ItemType.IMAGE_TEMP
    assert Path(item.path).suffix.lower() == ".png"
    assert Path(item.path).exists()
