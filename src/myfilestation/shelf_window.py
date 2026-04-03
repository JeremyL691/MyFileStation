import os
from typing import Dict, List, Optional

from PySide6 import QtCore, QtGui, QtWidgets

import win32api
import win32con

from .controller import ShelfController
from .models import ItemType, StationItem
from .settings import AppSettings
from .utils import (
    create_app_icon,
    create_line_icon,
    get_file_icon,
    open_in_explorer_select,
    open_with_default_app,
)


def _mime_is_supported(mime: QtCore.QMimeData) -> bool:
    return mime.hasUrls() or mime.hasText() or mime.hasImage()


def _item_type_label(item: StationItem) -> str:
    if item.item_type == ItemType.TEXT_TEMP:
        return "TEXT"
    if item.item_type == ItemType.IMAGE_TEMP:
        return "IMAGE"
    return "FILE"


def _item_meta_text(item: StationItem) -> str:
    labels = []
    if item.item_type == ItemType.FILE:
        labels.append("Local file")
    elif item.item_type == ItemType.TEXT_TEMP:
        labels.append("Clipboard text")
    elif item.item_type == ItemType.IMAGE_TEMP:
        labels.append("Clipboard image")

    if item.is_pinned:
        labels.append("Pinned")

    if not item.exists:
        labels.append("Missing")

    return " | ".join(labels)


class ElidedLabel(QtWidgets.QLabel):
    def __init__(self, text: str = "", parent=None, elide_mode=QtCore.Qt.ElideRight):
        super().__init__(text, parent)
        self._full_text = text or ""
        self._elide_mode = elide_mode
        self.setMinimumWidth(0)
        self.setSizePolicy(QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Preferred)

    def set_full_text(self, text: str) -> None:
        self._full_text = text or ""
        self._update_elide()

    def resizeEvent(self, event: QtGui.QResizeEvent) -> None:
        super().resizeEvent(event)
        self._update_elide()

    def _update_elide(self) -> None:
        fm = self.fontMetrics()
        super().setText(fm.elidedText(self._full_text, self._elide_mode, max(0, self.width())))


class ShelfListWidget(QtWidgets.QListWidget):
    request_remove_item = QtCore.Signal(object)
    dropped_mime = QtCore.Signal(object)
    drag_completed = QtCore.Signal(list, bool)
    invalid_drag_items = QtCore.Signal(list)
    request_preview_selection = QtCore.Signal()
    request_delete_selection = QtCore.Signal()

    def __init__(self) -> None:
        super().__init__()
        self._drag_start_pos = QtCore.QPoint()

        self.setSelectionMode(QtWidgets.QAbstractItemView.ExtendedSelection)
        self.setDragEnabled(True)
        self.setAcceptDrops(True)
        self.setDropIndicatorShown(True)
        self.setDefaultDropAction(QtCore.Qt.CopyAction)
        self.viewport().setAcceptDrops(True)
        self.setSpacing(8)
        self.setFrameShape(QtWidgets.QFrame.NoFrame)

    def mousePressEvent(self, event: QtGui.QMouseEvent) -> None:
        if event.button() == QtCore.Qt.MiddleButton:
            item = self.itemAt(event.position().toPoint())
            if item:
                station_item = item.data(QtCore.Qt.UserRole)
                if station_item and not station_item.is_pinned:
                    self.request_remove_item.emit(station_item)
            return

        self._drag_start_pos = event.position().toPoint()
        super().mousePressEvent(event)

    def mouseMoveEvent(self, event: QtGui.QMouseEvent) -> None:
        if not (event.buttons() & QtCore.Qt.LeftButton):
            return super().mouseMoveEvent(event)

        if (event.position().toPoint() - self._drag_start_pos).manhattanLength() < 6:
            return super().mouseMoveEvent(event)

        selected = self.selectedItems()
        paths = []
        station_item_ids = []
        missing_items = []

        for list_item in selected:
            station_item = list_item.data(QtCore.Qt.UserRole)
            if not station_item:
                continue
            if os.path.exists(station_item.path):
                paths.append(station_item.path)
                station_item_ids.append(station_item.id)
            else:
                missing_items.append(station_item)

        if missing_items:
            self.invalid_drag_items.emit(missing_items)

        if not paths:
            return

        mime = QtCore.QMimeData()
        mime.setUrls([QtCore.QUrl.fromLocalFile(path) for path in paths])

        drag = QtGui.QDrag(self)
        drag.setMimeData(mime)
        result = drag.exec(QtCore.Qt.CopyAction | QtCore.Qt.MoveAction)
        self.drag_completed.emit(station_item_ids, result != QtCore.Qt.IgnoreAction)

    def dragEnterEvent(self, event: QtGui.QDragEnterEvent) -> None:
        if _mime_is_supported(event.mimeData()):
            event.acceptProposedAction()
        else:
            event.ignore()

    def dragMoveEvent(self, event: QtGui.QDragMoveEvent) -> None:
        if _mime_is_supported(event.mimeData()):
            event.acceptProposedAction()
        else:
            event.ignore()

    def dropEvent(self, event: QtGui.QDropEvent) -> None:
        if _mime_is_supported(event.mimeData()):
            self.dropped_mime.emit(event.mimeData())
            event.acceptProposedAction()
        else:
            event.ignore()

    def keyPressEvent(self, event: QtGui.QKeyEvent) -> None:
        if event.key() in (QtCore.Qt.Key_Return, QtCore.Qt.Key_Enter, QtCore.Qt.Key_Space):
            self.request_preview_selection.emit()
            event.accept()
            return
        if event.key() == QtCore.Qt.Key_Delete:
            self.request_delete_selection.emit()
            event.accept()
            return
        super().keyPressEvent(event)


class EmptyStateWidget(QtWidgets.QFrame):
    def __init__(self) -> None:
        super().__init__()
        self.setObjectName("EmptyState")
        self.setStyleSheet(
            """
            QFrame#EmptyState {
                background: rgba(255,255,255,0.03);
                border: 1px dashed rgba(255,255,255,0.14);
                border-radius: 14px;
            }
            QLabel#EmptyTitle {
                color: #F4F7FC;
                font-size: 16px;
                font-weight: 600;
            }
            QLabel#EmptyBody {
                color: rgba(230,236,248,0.78);
                font-size: 12px;
            }
            QLabel#EmptyTips {
                color: rgba(163,180,208,0.88);
                font-size: 11px;
            }
            """
        )

        layout = QtWidgets.QVBoxLayout(self)
        layout.setContentsMargins(22, 22, 22, 22)
        layout.setSpacing(12)
        layout.setAlignment(QtCore.Qt.AlignCenter)

        icon_label = QtWidgets.QLabel()
        icon_label.setPixmap(create_app_icon(56).pixmap(56, 56))
        icon_label.setAlignment(QtCore.Qt.AlignCenter)

        title = QtWidgets.QLabel("Shelf Is Empty")
        title.setObjectName("EmptyTitle")
        title.setAlignment(QtCore.Qt.AlignCenter)

        body = QtWidgets.QLabel(
            "Drop files from Explorer, paste text or images with Ctrl+V, "
            "or open the shelf from the tray when you need a temporary hand-off space."
        )
        body.setObjectName("EmptyBody")
        body.setWordWrap(True)
        body.setAlignment(QtCore.Qt.AlignCenter)

        tips = QtWidgets.QLabel(
            "Edge reveal currently watches Explorer and Desktop drags.\n"
            "Shortcuts: Ctrl+V paste, Enter open, Delete remove, Esc hide."
        )
        tips.setObjectName("EmptyTips")
        tips.setWordWrap(True)
        tips.setAlignment(QtCore.Qt.AlignCenter)

        layout.addWidget(icon_label)
        layout.addWidget(title)
        layout.addWidget(body)
        layout.addWidget(tips)


class ShelfItemCard(QtWidgets.QFrame):
    remove_requested = QtCore.Signal(object)
    pin_toggled = QtCore.Signal(object, bool)

    def __init__(self, item: StationItem, icon: QtGui.QPixmap) -> None:
        super().__init__()
        self.item = item
        self.setObjectName("ShelfItemCard")
        self.setProperty("selected", False)
        self.setProperty("pinned", item.is_pinned)
        self.setAttribute(QtCore.Qt.WA_Hover, True)
        self.setStyleSheet(
            """
            QFrame#ShelfItemCard {
                background: rgba(255,255,255,0.07);
                border: 1px solid rgba(255,255,255,0.08);
                border-radius: 14px;
            }
            QFrame#ShelfItemCard[selected="true"] {
                background: rgba(73,116,191,0.30);
                border: 1px solid rgba(138,177,255,0.72);
            }
            QFrame#ShelfItemCard[pinned="true"] {
                border-color: rgba(158,194,255,0.78);
            }
            QLabel#ItemName {
                color: #F7FAFF;
                font-size: 13px;
                font-weight: 600;
            }
            QLabel#ItemMeta {
                color: rgba(205,218,241,0.82);
                font-size: 11px;
            }
            QLabel#ItemPath {
                color: rgba(167,182,207,0.84);
                font-size: 11px;
            }
            QLabel#ItemBadge {
                color: #DCE8FF;
                background: rgba(255,255,255,0.08);
                border: 1px solid rgba(255,255,255,0.10);
                border-radius: 8px;
                padding: 2px 6px;
                font-size: 10px;
                font-weight: 600;
            }
            QLabel#ItemBadge[pinned="true"] {
                color: #EAF1FF;
                background: rgba(122,167,255,0.22);
                border-color: rgba(150,186,255,0.55);
            }
            QLabel#ThumbFrame {
                background: rgba(10,14,20,0.36);
                border: 1px solid rgba(255,255,255,0.08);
                border-radius: 12px;
            }
            QToolButton#ItemActionButton {
                background: rgba(255,255,255,0.04);
                border: 1px solid rgba(255,255,255,0.08);
                border-radius: 8px;
                padding: 4px;
            }
            QToolButton#ItemActionButton:hover {
                background: rgba(255,255,255,0.12);
                border-color: rgba(255,255,255,0.18);
            }
            """
        )

        layout = QtWidgets.QHBoxLayout(self)
        layout.setContentsMargins(10, 10, 10, 10)
        layout.setSpacing(10)

        thumb_frame = QtWidgets.QLabel()
        thumb_frame.setObjectName("ThumbFrame")
        thumb_frame.setFixedSize(52, 52)
        thumb_frame.setAlignment(QtCore.Qt.AlignCenter)
        thumb_frame.setPixmap(icon)

        info_col = QtWidgets.QVBoxLayout()
        info_col.setContentsMargins(0, 0, 0, 0)
        info_col.setSpacing(4)

        title_row = QtWidgets.QHBoxLayout()
        title_row.setContentsMargins(0, 0, 0, 0)
        title_row.setSpacing(8)

        self.name_label = ElidedLabel("", elide_mode=QtCore.Qt.ElideRight)
        self.name_label.setObjectName("ItemName")
        self.name_label.set_full_text(item.display_name)
        self.name_label.setToolTip(item.display_name)

        self.badge_label = QtWidgets.QLabel(_item_type_label(item))
        self.badge_label.setObjectName("ItemBadge")
        self.badge_label.setProperty("pinned", item.is_pinned)
        self.badge_label.setAlignment(QtCore.Qt.AlignCenter)

        title_row.addWidget(self.name_label, 1)
        title_row.addWidget(self.badge_label, 0)

        self.meta_label = QtWidgets.QLabel(_item_meta_text(item))
        self.meta_label.setObjectName("ItemMeta")

        self.path_label = ElidedLabel("", elide_mode=QtCore.Qt.ElideMiddle)
        self.path_label.setObjectName("ItemPath")
        self.path_label.set_full_text(item.path)
        self.path_label.setToolTip(item.path)

        info_col.addLayout(title_row)
        info_col.addWidget(self.meta_label)
        info_col.addWidget(self.path_label)

        action_col = QtWidgets.QVBoxLayout()
        action_col.setContentsMargins(0, 0, 0, 0)
        action_col.setSpacing(6)

        self.pin_btn = QtWidgets.QToolButton()
        self.pin_btn.setObjectName("ItemActionButton")
        self.pin_btn.setCheckable(True)
        self.pin_btn.setChecked(item.is_pinned)
        self.pin_btn.clicked.connect(self._on_pin_clicked)

        self.remove_btn = QtWidgets.QToolButton()
        self.remove_btn.setObjectName("ItemActionButton")
        self.remove_btn.setIcon(create_line_icon("close", 16))
        self.remove_btn.setIconSize(QtCore.QSize(16, 16))
        self.remove_btn.setToolTip("Remove from shelf")
        self.remove_btn.clicked.connect(lambda: self.remove_requested.emit(self.item))

        action_col.addWidget(self.pin_btn, 0, QtCore.Qt.AlignRight)
        action_col.addWidget(self.remove_btn, 0, QtCore.Qt.AlignRight)
        action_col.addStretch(1)

        layout.addWidget(thumb_frame, 0, QtCore.Qt.AlignTop)
        layout.addLayout(info_col, 1)
        layout.addLayout(action_col)

        self._refresh_pin_state()

    def set_selected(self, selected: bool) -> None:
        self.setProperty("selected", selected)
        self._repolish()

    def _on_pin_clicked(self, checked: bool) -> None:
        self.pin_toggled.emit(self.item, checked)

    def update_item_state(self) -> None:
        self.setProperty("pinned", self.item.is_pinned)
        self.badge_label.setProperty("pinned", self.item.is_pinned)
        self.pin_btn.setChecked(self.item.is_pinned)
        self.meta_label.setText(_item_meta_text(self.item))
        self._refresh_pin_state()
        self._repolish()

    def _refresh_pin_state(self) -> None:
        icon_name = "pin-filled" if self.item.is_pinned else "pin"
        self.pin_btn.setIcon(create_line_icon(icon_name, 16, "#DCE8FF"))
        self.pin_btn.setIconSize(QtCore.QSize(16, 16))
        self.pin_btn.setToolTip(
            "Pinned: protected from auto-remove and bulk clear"
            if self.item.is_pinned
            else "Pin item"
        )

    def _repolish(self) -> None:
        for widget in (self, self.badge_label):
            widget.style().unpolish(widget)
            widget.style().polish(widget)
            widget.update()


class ShelfWindow(QtWidgets.QWidget):
    hidden_signal = QtCore.Signal()

    def __init__(self, settings: AppSettings, controller: ShelfController) -> None:
        super().__init__()
        self.settings = settings
        self.controller = controller
        self._item_cards: Dict[str, ShelfItemCard] = {}
        self._shown_by_edge_drag = False

        self.setAcceptDrops(True)
        self.setWindowFlags(
            QtCore.Qt.Tool
            | QtCore.Qt.FramelessWindowHint
            | QtCore.Qt.WindowStaysOnTopHint
        )
        self.setAttribute(QtCore.Qt.WA_TranslucentBackground, True)

        self._build_ui()
        self._setup_shortcuts()

        self._anim = QtCore.QPropertyAnimation(self, b"windowOpacity")
        self._anim.setDuration(180)

        self._watchdog = QtCore.QTimer(self)
        self._watchdog.setInterval(80)
        self._watchdog.timeout.connect(self._watch_drag_cancel)

        self.setWindowOpacity(0.0)
        self.hide()
        self.reposition()
        self._refresh_ui_state()

    def hideEvent(self, event: QtGui.QHideEvent) -> None:
        super().hideEvent(event)
        self._watchdog.stop()
        self._shown_by_edge_drag = False
        self.hidden_signal.emit()

    def dragEnterEvent(self, event: QtGui.QDragEnterEvent) -> None:
        if _mime_is_supported(event.mimeData()):
            event.acceptProposedAction()
        else:
            event.ignore()

    def dragMoveEvent(self, event: QtGui.QDragMoveEvent) -> None:
        if _mime_is_supported(event.mimeData()):
            event.acceptProposedAction()
        else:
            event.ignore()

    def dropEvent(self, event: QtGui.QDropEvent) -> None:
        if _mime_is_supported(event.mimeData()):
            self._handle_dropped_mime(event.mimeData())
            event.acceptProposedAction()
        else:
            event.ignore()

    def _build_ui(self) -> None:
        self.setStyleSheet(
            """
            QWidget {
                color: #F7FAFF;
                font-family: "Segoe UI";
            }
            QFrame#ShelfCard {
                background: rgba(18,23,31,0.94);
                border: 1px solid rgba(255,255,255,0.08);
                border-radius: 18px;
            }
            QLabel#HeaderTitle {
                color: #F4F7FC;
                font-size: 16px;
                font-weight: 700;
            }
            QLabel#HeaderSubtitle {
                color: rgba(188,203,229,0.78);
                font-size: 11px;
            }
            QLabel#StatusLabel {
                color: rgba(205,218,241,0.84);
                font-size: 11px;
            }
            QPushButton#SecondaryButton, QToolButton#ChromeButton {
                color: #F7FAFF;
                background: rgba(255,255,255,0.04);
                border: 1px solid rgba(255,255,255,0.08);
                border-radius: 10px;
                padding: 8px 12px;
            }
            QPushButton#SecondaryButton:hover, QToolButton#ChromeButton:hover {
                background: rgba(255,255,255,0.10);
                border-color: rgba(255,255,255,0.18);
            }
            QListWidget {
                background: transparent;
                border: 0;
                outline: none;
            }
            QListWidget::item {
                background: transparent;
                border: 0;
                margin: 0;
            }
            """
        )

        root = QtWidgets.QVBoxLayout(self)
        root.setContentsMargins(10, 10, 10, 10)
        root.setSpacing(0)

        self.card = QtWidgets.QFrame()
        self.card.setObjectName("ShelfCard")
        card_layout = QtWidgets.QVBoxLayout(self.card)
        card_layout.setContentsMargins(14, 14, 14, 14)
        card_layout.setSpacing(12)

        header = QtWidgets.QHBoxLayout()
        header.setContentsMargins(0, 0, 0, 0)
        header.setSpacing(10)

        icon_label = QtWidgets.QLabel()
        icon_label.setPixmap(create_app_icon(28).pixmap(28, 28))

        title_col = QtWidgets.QVBoxLayout()
        title_col.setContentsMargins(0, 0, 0, 0)
        title_col.setSpacing(1)

        title = QtWidgets.QLabel("MyFileStation")
        title.setObjectName("HeaderTitle")
        subtitle = QtWidgets.QLabel("Quick hand-off shelf for local files")
        subtitle.setObjectName("HeaderSubtitle")

        title_col.addWidget(title)
        title_col.addWidget(subtitle)

        self.btn_close = QtWidgets.QToolButton()
        self.btn_close.setObjectName("ChromeButton")
        self.btn_close.setIcon(create_line_icon("close", 18))
        self.btn_close.setIconSize(QtCore.QSize(18, 18))
        self.btn_close.setFixedSize(34, 34)
        self.btn_close.setToolTip("Hide shelf")
        self.btn_close.clicked.connect(self.hide_soft)

        header.addWidget(icon_label, 0, QtCore.Qt.AlignTop)
        header.addLayout(title_col, 1)
        header.addWidget(self.btn_close, 0, QtCore.Qt.AlignTop)
        card_layout.addLayout(header)

        self.content_stack = QtWidgets.QStackedWidget()
        self.content_stack.setContentsMargins(0, 0, 0, 0)

        self.empty_state = EmptyStateWidget()

        list_page = QtWidgets.QWidget()
        list_layout = QtWidgets.QVBoxLayout(list_page)
        list_layout.setContentsMargins(0, 0, 0, 0)
        list_layout.setSpacing(0)

        self.list = ShelfListWidget()
        self.list.request_remove_item.connect(self.remove_item)
        self.list.dropped_mime.connect(self._handle_dropped_mime)
        self.list.drag_completed.connect(self._handle_drag_completed)
        self.list.invalid_drag_items.connect(self._handle_invalid_drag_items)
        self.list.request_preview_selection.connect(self.preview_selected)
        self.list.request_delete_selection.connect(self.remove_selected_unlocked)
        self.list.setContextMenuPolicy(QtCore.Qt.CustomContextMenu)
        self.list.customContextMenuRequested.connect(self._show_context_menu)
        self.list.itemSelectionChanged.connect(self._on_selection_changed)
        self.list.itemDoubleClicked.connect(self._preview_list_item)
        self.list.itemActivated.connect(self._preview_list_item)
        list_layout.addWidget(self.list)

        self.content_stack.addWidget(self.empty_state)
        self.content_stack.addWidget(list_page)
        card_layout.addWidget(self.content_stack, 1)

        footer = QtWidgets.QHBoxLayout()
        footer.setContentsMargins(0, 0, 0, 0)
        footer.setSpacing(8)

        self.status_label = QtWidgets.QLabel()
        self.status_label.setObjectName("StatusLabel")
        self.status_label.setWordWrap(True)

        self.btn_select_all = QtWidgets.QPushButton("Select All")
        self.btn_select_all.setObjectName("SecondaryButton")
        self.btn_select_all.clicked.connect(self.list.selectAll)

        self.btn_clear = QtWidgets.QPushButton("Clear Unlocked")
        self.btn_clear.setObjectName("SecondaryButton")
        self.btn_clear.clicked.connect(self.clear_unlocked)

        footer.addWidget(self.status_label, 1)
        footer.addWidget(self.btn_select_all)
        footer.addWidget(self.btn_clear)
        card_layout.addLayout(footer)

        root.addWidget(self.card)

    def _setup_shortcuts(self) -> None:
        self._shortcuts: List[QtGui.QShortcut] = []
        shortcuts = (
            ("Ctrl+V", self.import_from_clipboard),
            ("Ctrl+C", self.export_selection_to_clipboard),
            ("Ctrl+A", self.list.selectAll),
            ("Space", self.preview_selected),
            ("Return", self.preview_selected),
            ("Enter", self.preview_selected),
            ("Delete", self.remove_selected_unlocked),
            ("Esc", self.hide_soft),
        )
        for key, callback in shortcuts:
            shortcut = QtGui.QShortcut(QtGui.QKeySequence(key), self)
            shortcut.setContext(QtCore.Qt.WindowShortcut)
            shortcut.activated.connect(callback)
            self._shortcuts.append(shortcut)

    def _is_left_button_down(self) -> bool:
        return (win32api.GetAsyncKeyState(win32con.VK_LBUTTON) & 0x8000) != 0

    def _watch_drag_cancel(self) -> None:
        if not self._shown_by_edge_drag:
            return
        if not self._is_left_button_down():
            if self.list.count() == 0:
                self.hide_soft()
            self._shown_by_edge_drag = False
            self._watchdog.stop()

    def reposition(self) -> None:
        screen = QtGui.QGuiApplication.screenAt(QtGui.QCursor.pos())
        if not screen:
            screen = QtGui.QGuiApplication.primaryScreen()
        if not screen:
            return

        rect = screen.availableGeometry()
        margin = 8
        desired_w = 388
        desired_h = 660

        width = min(desired_w, max(260, rect.width() - margin * 2))
        height = min(desired_h, max(300, rect.height() - margin * 2))

        if self.settings.dock_side == "left":
            x = rect.x() + margin
        else:
            x = rect.x() + rect.width() - width - margin

        y = rect.y() + margin
        self.setGeometry(x, y, width, height)

    def show_soft(self) -> None:
        self._shown_by_edge_drag = False
        self._watchdog.stop()

        self.reposition()
        self.show()
        self.raise_()
        self.activateWindow()

        self._anim.stop()
        self._anim.setStartValue(self.windowOpacity())
        self._anim.setEndValue(1.0)
        self._anim.start()

        target = self.list if self.list.count() else self.card
        QtCore.QTimer.singleShot(0, target.setFocus)

    def show_from_edge_drag(self) -> None:
        self._shown_by_edge_drag = True
        self._watchdog.start()

        self.reposition()
        if not self.isVisible():
            self.show()
            self.raise_()

        self._anim.stop()
        self._anim.setStartValue(self.windowOpacity())
        self._anim.setEndValue(1.0)
        self._anim.start()

    def hide_soft(self) -> None:
        if not self.isVisible() and self.windowOpacity() <= 0.01:
            return
        self._anim.stop()
        self._anim.setStartValue(self.windowOpacity())
        self._anim.setEndValue(0.0)
        self._anim.finished.connect(self._really_hide_once)
        self._anim.start()

    def _really_hide_once(self) -> None:
        try:
            self._anim.finished.disconnect(self._really_hide_once)
        except Exception:
            pass
        if self.windowOpacity() <= 0.01:
            self.hide()

    def refresh_status_text(self) -> None:
        count = self.list.count()
        selected_count = len(self.list.selectedItems())
        auto_remove = "Auto-remove after drag-out is On" if self.settings.remove_after_drag_out else "Auto-remove after drag-out is Off"

        if count == 0:
            self.status_label.setText(f"{auto_remove}. Pinned items stay until you remove them manually.")
            return

        item_text = f"{count} item" if count == 1 else f"{count} items"
        selection_text = f" | {selected_count} selected" if selected_count else ""
        self.status_label.setText(f"{item_text}{selection_text} | {auto_remove}")

    def _refresh_ui_state(self) -> None:
        has_items = self.list.count() > 0
        self.content_stack.setCurrentIndex(1 if has_items else 0)
        self.btn_select_all.setEnabled(has_items)
        self.btn_clear.setEnabled(any(not item.is_pinned for item in self.controller.items))
        self.refresh_status_text()
        self._sync_item_card_states()

    def _sync_item_card_states(self) -> None:
        for index in range(self.list.count()):
            list_item = self.list.item(index)
            station_item = list_item.data(QtCore.Qt.UserRole)
            card = self._item_cards.get(station_item.id) if station_item else None
            if card:
                card.set_selected(list_item.isSelected())

    def _handle_dropped_mime(self, mime: QtCore.QMimeData) -> None:
        imported_any = False

        try:
            if mime.hasUrls():
                for url in mime.urls():
                    if url.isLocalFile():
                        self.add_file(url.toLocalFile())
                        imported_any = True

            elif mime.hasText():
                text = mime.text()
                if text and text.strip():
                    self.add_temp_text(text)
                    imported_any = True

            elif mime.hasImage():
                image = mime.imageData()
                if isinstance(image, QtGui.QImage) and not image.isNull():
                    self.add_temp_image(image)
                    imported_any = True
        except RuntimeError as exc:
            self._show_warning("Import Failed", str(exc))

        if imported_any:
            self._shown_by_edge_drag = False
            self._watchdog.stop()
            self.show_soft()

    def add_file(self, path: str) -> None:
        try:
            item = self.controller.add_file(path)
        except FileNotFoundError:
            self._show_warning("File Missing", f"The file is no longer available:\n\n{path}")
            return
        self._append_item(item)

    def add_temp_text(self, text: str) -> None:
        self._append_item(self.controller.add_temp_text(text))

    def add_temp_image(self, qimage: QtGui.QImage) -> None:
        self._append_item(self.controller.add_temp_image(qimage))

    def _append_item(self, item: StationItem) -> None:
        list_item = QtWidgets.QListWidgetItem()
        list_item.setData(QtCore.Qt.UserRole, item)
        list_item.setSizeHint(QtCore.QSize(320, 82))

        card = ShelfItemCard(item, self._pixmap_for_item(item))
        card.remove_requested.connect(self.force_remove_item)
        card.pin_toggled.connect(self._set_item_pinned)

        self._item_cards[item.id] = card
        self.list.addItem(list_item)
        self.list.setItemWidget(list_item, card)
        self.list.setCurrentItem(list_item)
        list_item.setSelected(True)
        self._refresh_ui_state()

    def _pixmap_for_item(self, item: StationItem) -> QtGui.QPixmap:
        if item.thumbnail_path and os.path.exists(item.thumbnail_path):
            pixmap = QtGui.QPixmap(item.thumbnail_path)
            if not pixmap.isNull():
                return pixmap.scaled(
                    48,
                    48,
                    QtCore.Qt.KeepAspectRatioByExpanding,
                    QtCore.Qt.SmoothTransformation,
                )

        if item.item_type == ItemType.TEXT_TEMP:
            return create_line_icon("text", 30).pixmap(30, 30)
        if item.item_type == ItemType.IMAGE_TEMP:
            return create_line_icon("image", 30).pixmap(30, 30)
        return get_file_icon(item.path).pixmap(34, 34)

    def _set_item_pinned(self, item: StationItem, pinned: bool) -> None:
        updated = self.controller.set_pinned(item.id, pinned)
        card = self._item_cards.get(updated.id)
        if card:
            card.update_item_state()
        self._refresh_ui_state()

    def _selected_station_items(self) -> List[StationItem]:
        selected = []
        for list_item in self.list.selectedItems():
            station_item = list_item.data(QtCore.Qt.UserRole)
            if station_item:
                selected.append(station_item)
        return selected

    def _first_selected_station_item(self) -> Optional[StationItem]:
        selected = self._selected_station_items()
        return selected[0] if selected else None

    def remove_item(self, station_item: StationItem) -> None:
        removed_item = self.controller.remove_item(station_item.id, force=False)
        if removed_item is not None:
            self._remove_item_widget(removed_item.id)

    def force_remove_item(self, station_item: StationItem) -> None:
        removed_item = self.controller.remove_item(station_item.id, force=True)
        if removed_item is not None:
            self._remove_item_widget(removed_item.id)

    def remove_selected_unlocked(self) -> None:
        removable = [item for item in self._selected_station_items() if not item.is_pinned]
        for station_item in removable:
            self.force_remove_item(station_item)

    def clear_unlocked(self) -> None:
        for station_item in self.controller.clear_unlocked():
            self._remove_item_widget(station_item.id)

    def _remove_item_widget(self, item_id: str) -> None:
        self._item_cards.pop(item_id, None)

        for index in range(self.list.count()):
            list_item = self.list.item(index)
            current = list_item.data(QtCore.Qt.UserRole)
            if current and current.id == item_id:
                self.list.takeItem(index)
                break

        self._refresh_ui_state()
        if self.list.count() == 0:
            self.hide_soft()

    def _show_context_menu(self, pos: QtCore.QPoint) -> None:
        list_item = self.list.itemAt(pos)
        if not list_item:
            return

        station_item = list_item.data(QtCore.Qt.UserRole)
        if not station_item:
            return

        menu = QtWidgets.QMenu(self)
        open_action = menu.addAction("Open")
        open_folder_action = menu.addAction("Open File Location")
        copy_path_action = menu.addAction("Copy File Path")
        menu.addSeparator()
        pin_action = menu.addAction("Unpin Item" if station_item.is_pinned else "Pin Item")
        remove_action = menu.addAction("Remove")
        remove_action.setEnabled(not station_item.is_pinned)
        force_remove_action = menu.addAction("Force Remove")

        action = self._exec_menu(menu, pos)
        if action == open_action:
            self._open_item(station_item)
        elif action == open_folder_action:
            self._reveal_item(station_item)
        elif action == copy_path_action:
            self._copy_item_path(station_item)
        elif action == pin_action:
            self._set_item_pinned(station_item, not station_item.is_pinned)
        elif action == remove_action:
            self.remove_item(station_item)
        elif action == force_remove_action:
            self.force_remove_item(station_item)

    def import_from_clipboard(self) -> None:
        clipboard = QtGui.QGuiApplication.clipboard()
        mime = clipboard.mimeData()

        try:
            if mime and mime.hasUrls():
                imported = False
                for url in mime.urls():
                    if url.isLocalFile():
                        self.add_file(url.toLocalFile())
                        imported = True
                if imported:
                    self.show_soft()
                    return

            image = clipboard.image()
            if not image.isNull():
                self.add_temp_image(image)
                self.show_soft()
                return

            text = clipboard.text()
            if text and text.strip():
                self.add_temp_text(text)
                self.show_soft()
        except RuntimeError as exc:
            self._show_warning("Clipboard Import Failed", str(exc))

    def export_selection_to_clipboard(self) -> None:
        selected_items = self._selected_station_items()
        paths, missing_items = self.controller.existing_paths_for_ids([item.id for item in selected_items])

        if missing_items:
            self._warn_and_prune_missing_items(missing_items, "copy to the clipboard")

        if not paths:
            return

        mime = QtCore.QMimeData()
        mime.setUrls([QtCore.QUrl.fromLocalFile(path) for path in paths])
        QtGui.QGuiApplication.clipboard().setMimeData(mime)

    def _preview_list_item(self, list_item: QtWidgets.QListWidgetItem) -> None:
        station_item = list_item.data(QtCore.Qt.UserRole)
        if station_item:
            self._open_item(station_item)

    def preview_selected(self) -> None:
        station_item = self._first_selected_station_item()
        if station_item:
            self._open_item(station_item)

    def _on_selection_changed(self) -> None:
        self._sync_item_card_states()
        self.refresh_status_text()

    def _handle_drag_completed(self, item_ids: List[str], success: bool) -> None:
        for station_item in self.controller.handle_drag_out_result(item_ids, success):
            self._remove_item_widget(station_item.id)

    def _handle_invalid_drag_items(self, items: List[StationItem]) -> None:
        self._warn_and_prune_missing_items(items, "drag out")

    def _copy_item_path(self, item: StationItem) -> None:
        if not item.exists:
            self._warn_and_prune_missing_items([item], "copy its path")
            return
        QtGui.QGuiApplication.clipboard().setText(item.path)

    def _open_item(self, item: StationItem) -> None:
        if not item.exists:
            self._warn_and_prune_missing_items([item], "open")
            return
        try:
            open_with_default_app(item.path)
        except RuntimeError as exc:
            self._show_warning("Open Failed", str(exc))

    def _reveal_item(self, item: StationItem) -> None:
        if not item.exists:
            self._warn_and_prune_missing_items([item], "reveal in Explorer")
            return
        try:
            open_in_explorer_select(item.path)
        except RuntimeError as exc:
            self._show_warning("Explorer Failed", str(exc))

    def _warn_and_prune_missing_items(self, items: List[StationItem], action_label: str) -> None:
        if not items:
            return

        unique_items = []
        seen = set()
        for item in items:
            if item.id in seen:
                continue
            seen.add(item.id)
            unique_items.append(item)

        label = unique_items[0].display_name
        if len(unique_items) > 1:
            label = f"{len(unique_items)} selected items"

        self._show_warning(
            "File Missing",
            f"{label} could not be used because the backing file is no longer available.\n\n"
            f"The stale shelf entry was removed before trying to {action_label}.",
        )

        for item in unique_items:
            self.controller.remove_missing_item(item.id)
            self._remove_item_widget(item.id)

    def _show_warning(self, title: str, message: str) -> None:
        QtWidgets.QMessageBox.warning(self, title, message)

    def _exec_menu(self, menu: QtWidgets.QMenu, pos: QtCore.QPoint):
        return menu.exec(self.list.mapToGlobal(pos))
