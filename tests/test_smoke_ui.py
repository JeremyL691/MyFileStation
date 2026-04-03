import pytest
from PySide6 import QtCore, QtWidgets

from myfilestation.controller import ShelfController
from myfilestation.settings import AppSettings
from myfilestation.shelf_window import ShelfWindow


def _build_window(qtbot, tmp_path, monkeypatch):
    monkeypatch.setenv("TEMP", str(tmp_path))
    settings = AppSettings()
    controller = ShelfController(settings)
    window = ShelfWindow(settings, controller)
    qtbot.addWidget(window)
    window.show()
    qtbot.waitUntil(window.isVisible)
    return window, controller


@pytest.mark.smoke
def test_empty_state_visible_by_default(qtbot, tmp_path, monkeypatch):
    window, _ = _build_window(qtbot, tmp_path, monkeypatch)

    assert window.content_stack.currentWidget() is window.empty_state
    assert window.btn_select_all.isEnabled() is False


@pytest.mark.smoke
def test_adding_item_switches_to_list_state_and_updates_status(qtbot, tmp_path, monkeypatch):
    window, _ = _build_window(qtbot, tmp_path, monkeypatch)

    file_path = tmp_path / "sample.txt"
    file_path.write_text("demo", encoding="utf-8")
    window.add_file(str(file_path))

    assert window.content_stack.currentWidget() is not window.empty_state
    assert window.list.count() == 1
    assert "1 item" in window.status_label.text()


@pytest.mark.smoke
def test_delete_shortcut_removes_only_unpinned_items(qtbot, tmp_path, monkeypatch):
    window, controller = _build_window(qtbot, tmp_path, monkeypatch)

    first = tmp_path / "first.txt"
    first.write_text("a", encoding="utf-8")
    second = tmp_path / "second.txt"
    second.write_text("b", encoding="utf-8")

    window.add_file(str(first))
    window.add_file(str(second))

    second_item = controller.get_items()[1]
    controller.set_pinned(second_item.id, True)
    window._item_cards[second_item.id].update_item_state()

    window.list.selectAll()
    window.list.setFocus()
    qtbot.keyClick(window.list, QtCore.Qt.Key_Delete)

    remaining_ids = [window.list.item(index).data(QtCore.Qt.UserRole).id for index in range(window.list.count())]
    assert remaining_ids == [second_item.id]


@pytest.mark.smoke
def test_context_menu_open_action_routes_to_item_handler(qtbot, tmp_path, monkeypatch):
    window, _ = _build_window(qtbot, tmp_path, monkeypatch)

    file_path = tmp_path / "openme.txt"
    file_path.write_text("open", encoding="utf-8")
    window.add_file(str(file_path))

    called = {"item_id": None}

    def fake_open(item):
        called["item_id"] = item.id

    def fake_exec(menu, _pos):
        return menu.actions()[0]

    monkeypatch.setattr(window, "_open_item", fake_open)
    monkeypatch.setattr(window, "_exec_menu", fake_exec)

    item_rect = window.list.visualItemRect(window.list.item(0))
    window._show_context_menu(item_rect.center())

    assert called["item_id"] == window.list.item(0).data(QtCore.Qt.UserRole).id


@pytest.mark.smoke
def test_status_label_tracks_selection_count(qtbot, tmp_path, monkeypatch):
    window, _ = _build_window(qtbot, tmp_path, monkeypatch)

    for index in range(2):
        file_path = tmp_path / f"file-{index}.txt"
        file_path.write_text(str(index), encoding="utf-8")
        window.add_file(str(file_path))

    window.list.clearSelection()
    window.list.item(0).setSelected(True)
    window._on_selection_changed()

    assert "2 items" in window.status_label.text()
    assert "1 selected" in window.status_label.text()


@pytest.mark.smoke
def test_dragging_out_last_unpinned_item_hides_window(qtbot, tmp_path, monkeypatch):
    window, controller = _build_window(qtbot, tmp_path, monkeypatch)

    file_path = tmp_path / "drag-out.txt"
    file_path.write_text("demo", encoding="utf-8")
    window.add_file(str(file_path))

    item = controller.get_items()[0]
    window._handle_drag_completed([item.id], True)
    qtbot.waitUntil(lambda: not window.isVisible(), timeout=1000)

    assert window.list.count() == 0
