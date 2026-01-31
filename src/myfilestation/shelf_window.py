import os
from typing import List

from PySide6 import QtCore, QtGui, QtWidgets

import win32api
import win32con

from .models import StationItem, ItemType
from .settings import AppSettings
from .utils import (
    is_image_file,
    create_temp_text_file,
    create_temp_image_file_from_qimage,
    open_with_default_app,
    open_in_explorer_select,
)


class ElidedLabel(QtWidgets.QLabel):
    """QLabel that automatically adds '...' when text is too long."""
    def __init__(self, text: str = "", parent=None, elide_mode=QtCore.Qt.ElideRight):
        super().__init__(text, parent)
        self._full_text = text or ""
        self._elide_mode = elide_mode
        self.setMinimumWidth(0)
        self.setSizePolicy(QtWidgets.QSizePolicy.Expanding, QtWidgets.QSizePolicy.Preferred)

    def set_full_text(self, text: str) -> None:
        self._full_text = text or ""
        self._update_elide()

    def resizeEvent(self, event):
        super().resizeEvent(event)
        self._update_elide()

    def _update_elide(self) -> None:
        fm = self.fontMetrics()
        w = max(0, self.width())
        super().setText(fm.elidedText(self._full_text, self._elide_mode, w))


class ShelfListWidget(QtWidgets.QListWidget):
    request_remove_item = QtCore.Signal(object)  # StationItem
    dropped_mime = QtCore.Signal(object)         # QMimeData

    def __init__(self, settings: AppSettings) -> None:
        super().__init__()
        self.settings = settings

        self.setSelectionMode(QtWidgets.QAbstractItemView.ExtendedSelection)
        self.setDragEnabled(True)

        self.setAcceptDrops(True)
        self.setDropIndicatorShown(True)
        self.setDefaultDropAction(QtCore.Qt.CopyAction)
        self.viewport().setAcceptDrops(True)

        self._drag_start_pos = QtCore.QPoint()

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
        station_items = []

        for it in selected:
            s = it.data(QtCore.Qt.UserRole)
            if s and os.path.exists(s.path):
                paths.append(s.path)
                station_items.append(s)

        if not paths:
            return

        mime = QtCore.QMimeData()
        mime.setUrls([QtCore.QUrl.fromLocalFile(p) for p in paths])

        drag = QtGui.QDrag(self)
        drag.setMimeData(mime)
        result = drag.exec(QtCore.Qt.CopyAction | QtCore.Qt.MoveAction)

        # Remove-after-drag-out: ONLY remove UNLOCKED items
        if self.settings.remove_after_drag_out and result != QtCore.Qt.IgnoreAction:
            for s in station_items:
                if not s.is_pinned:  # locked items stay
                    self.request_remove_item.emit(s)

    def dragEnterEvent(self, event: QtGui.QDragEnterEvent) -> None:
        m = event.mimeData()
        if m.hasUrls() or m.hasText() or m.hasImage():
            event.acceptProposedAction()
        else:
            event.ignore()

    def dragMoveEvent(self, event: QtGui.QDragMoveEvent) -> None:
        m = event.mimeData()
        if m.hasUrls() or m.hasText() or m.hasImage():
            event.acceptProposedAction()
        else:
            event.ignore()

    def dropEvent(self, event: QtGui.QDropEvent) -> None:
        m = event.mimeData()
        if m.hasUrls() or m.hasText() or m.hasImage():
            self.dropped_mime.emit(m)
            event.acceptProposedAction()
        else:
            event.ignore()


class ShelfWindow(QtWidgets.QWidget):
    hidden_signal = QtCore.Signal()

    def __init__(self, settings: AppSettings) -> None:
        super().__init__()
        self.settings = settings
        self.items: List[StationItem] = []

        self.setAcceptDrops(True)
        self.setWindowFlags(
            QtCore.Qt.Tool
            | QtCore.Qt.FramelessWindowHint
            | QtCore.Qt.WindowStaysOnTopHint
        )
        self.setAttribute(QtCore.Qt.WA_TranslucentBackground, True)

        self._shown_by_edge_drag = False

        self._build_ui()
        self._setup_shortcuts()

        self._anim = QtCore.QPropertyAnimation(self, b"windowOpacity")
        self._anim.setDuration(180)

        # Drag-cancel watchdog
        self._watchdog = QtCore.QTimer(self)
        self._watchdog.setInterval(80)
        self._watchdog.timeout.connect(self._watch_drag_cancel)

        self.setWindowOpacity(0.0)
        self.hide()
        self.reposition()

    def hideEvent(self, event: QtGui.QHideEvent) -> None:
        super().hideEvent(event)
        self._watchdog.stop()
        self._shown_by_edge_drag = False
        self.hidden_signal.emit()

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

        r = screen.availableGeometry()
        margin = 8

        desired_w = 380
        desired_h = 640

        max_w = max(240, r.width() - margin * 2)
        max_h = max(260, r.height() - margin * 2)

        w = min(desired_w, max_w)
        h = min(desired_h, max_h)

        if self.settings.dock_side == "left":
            x = r.x() + margin
        else:
            x = r.x() + r.width() - w - margin

        y = r.y() + margin

        x = max(r.x() + margin, min(x, r.x() + r.width() - w - margin))
        y = max(r.y() + margin, min(y, r.y() + r.height() - h - margin))

        self.setGeometry(x, y, w, h)

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

    def show_from_edge_drag(self) -> None:
        self._shown_by_edge_drag = True
        self._watchdog.start()

        self.reposition()
        self.show()
        self.raise_()

        self._anim.stop()
        self._anim.setStartValue(self.windowOpacity())
        self._anim.setEndValue(1.0)
        self._anim.start()

    def hide_soft(self) -> None:
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

    def _build_ui(self) -> None:
        root = QtWidgets.QVBoxLayout(self)
        root.setContentsMargins(10, 10, 10, 10)
        root.setSpacing(8)

        self.card = QtWidgets.QFrame()
        self.card.setAcceptDrops(True)
        self.card.setStyleSheet("""
            QFrame { background: rgba(20,20,20,210); border-radius: 12px; }
            QLabel { color: white; }
            QPushButton { color: white; }
            QToolButton { color: white; }
        """)
        card_layout = QtWidgets.QVBoxLayout(self.card)
        card_layout.setContentsMargins(12, 12, 12, 12)
        card_layout.setSpacing(10)

        header = QtWidgets.QHBoxLayout()
        title = QtWidgets.QLabel("MyFileStation")
        title.setStyleSheet("font-size: 16px; font-weight: 600;")
        header.addWidget(title)
        header.addStretch(1)

        self.btn_close = QtWidgets.QPushButton("✕")
        self.btn_close.setFixedSize(36, 28)
        self.btn_close.clicked.connect(self.hide_soft)
        header.addWidget(self.btn_close)
        card_layout.addLayout(header)

        self.list = ShelfListWidget(self.settings)
        self.list.setStyleSheet("""
            QListWidget { background: transparent; border: 0px; }
            QListWidget::item { margin-bottom: 8px; }
        """)
        self.list.request_remove_item.connect(self.remove_item)
        self.list.dropped_mime.connect(self._handle_dropped_mime)

        self.list.setContextMenuPolicy(QtCore.Qt.CustomContextMenu)
        self.list.customContextMenuRequested.connect(self._show_context_menu)
        card_layout.addWidget(self.list, 1)

        # Footer: ONLY buttons
        footer = QtWidgets.QHBoxLayout()
        footer.setContentsMargins(0, 0, 0, 0)
        footer.setSpacing(8)

        self.btn_select_all = QtWidgets.QPushButton("Select All")
        self.btn_clear = QtWidgets.QPushButton("Clear (Unlocked)")
        self.btn_select_all.clicked.connect(self.list.selectAll)
        self.btn_clear.clicked.connect(self.clear_unlocked)

        footer.addWidget(self.btn_select_all)
        footer.addWidget(self.btn_clear)
        footer.addStretch(1)

        card_layout.addLayout(footer)
        root.addWidget(self.card)

    def _setup_shortcuts(self) -> None:
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+V"), self, activated=self.import_from_clipboard)
        QtGui.QShortcut(QtGui.QKeySequence("Ctrl+C"), self, activated=self.export_selection_to_clipboard)
        QtGui.QShortcut(QtGui.QKeySequence("Space"), self, activated=self.preview_selected)

    def _handle_dropped_mime(self, mime: QtCore.QMimeData) -> None:
        if mime.hasUrls():
            for url in mime.urls():
                if url.isLocalFile():
                    self.add_file(url.toLocalFile())
            self._shown_by_edge_drag = False
            self._watchdog.stop()
            self.show_soft()
            return

        if mime.hasText():
            txt = mime.text()
            if txt and txt.strip():
                self.add_temp_text(txt)
                self._shown_by_edge_drag = False
                self._watchdog.stop()
                self.show_soft()
            return

        if mime.hasImage():
            img = mime.imageData()
            if isinstance(img, QtGui.QImage) and not img.isNull():
                self.add_temp_image(img)
                self._shown_by_edge_drag = False
                self._watchdog.stop()
                self.show_soft()

    def add_file(self, path: str) -> None:
        if not os.path.exists(path):
            return
        name = os.path.basename(path)
        thumb = path if is_image_file(path) else None
        item = StationItem.new(ItemType.FILE, path, name, thumb)
        self._append_item(item)

    def add_temp_text(self, text: str) -> None:
        p = create_temp_text_file(text)
        name = os.path.basename(p)
        item = StationItem.new(ItemType.TEXT_TEMP, p, name, None)
        self._append_item(item)

    def add_temp_image(self, qimage: QtGui.QImage) -> None:
        p = create_temp_image_file_from_qimage(qimage)
        name = os.path.basename(p)
        item = StationItem.new(ItemType.IMAGE_TEMP, p, name, p)
        self._append_item(item)

    def _append_item(self, item: StationItem) -> None:
        self.items.append(item)
        lw_item = QtWidgets.QListWidgetItem()
        lw_item.setData(QtCore.Qt.UserRole, item)
        lw_item.setSizeHint(QtCore.QSize(320, 64))
        widget = self._make_item_widget(item)
        self.list.addItem(lw_item)
        self.list.setItemWidget(lw_item, widget)

    def _make_item_widget(self, item: StationItem) -> QtWidgets.QWidget:
        w = QtWidgets.QWidget()
        w.setStyleSheet("QWidget { background: rgba(255,255,255,35); border-radius: 10px; }")

        # We'll use a grid to place a top-right "X" button
        grid = QtWidgets.QGridLayout(w)
        grid.setContentsMargins(8, 8, 8, 8)
        grid.setHorizontalSpacing(10)
        grid.setVerticalSpacing(4)

        thumb = QtWidgets.QLabel()
        thumb.setFixedSize(44, 44)
        thumb.setStyleSheet("background: rgba(0,0,0,60); border-radius: 8px;")
        if item.thumbnail_path and os.path.exists(item.thumbnail_path):
            pm = QtGui.QPixmap(item.thumbnail_path)
            if not pm.isNull():
                thumb.setPixmap(
                    pm.scaled(44, 44, QtCore.Qt.KeepAspectRatioByExpanding, QtCore.Qt.SmoothTransformation)
                )
        grid.addWidget(thumb, 0, 0, 2, 1)

        name = ElidedLabel("", elide_mode=QtCore.Qt.ElideRight)
        name.setStyleSheet("font-weight: 600;")
        name.set_full_text(item.display_name)
        name.setToolTip(item.display_name)

        path = ElidedLabel("", elide_mode=QtCore.Qt.ElideMiddle)
        path.setStyleSheet("color: rgba(255,255,255,140); font-size: 11px;")
        path.set_full_text(item.path)
        path.setToolTip(item.path)

        grid.addWidget(name, 0, 1, 1, 1)
        grid.addWidget(path, 1, 1, 1, 1)

        # Right-side buttons container
        btn_col = QtWidgets.QVBoxLayout()
        btn_col.setContentsMargins(0, 0, 0, 0)
        btn_col.setSpacing(6)

        # "X" remove button (always allowed, even if locked)
        remove_btn = QtWidgets.QToolButton()
        remove_btn.setText("×")
        remove_btn.setFixedSize(26, 20)
        remove_btn.setToolTip("Remove from shelf")
        remove_btn.setStyleSheet("""
            QToolButton {
                border: 1px solid rgba(255,255,255,70);
                background: rgba(0,0,0,15);
                border-radius: 6px;
                font-size: 14px;
                font-weight: 700;
                padding: 0px;
            }
            QToolButton:hover {
                background: rgba(255,80,80,80);
            }
        """)
        remove_btn.clicked.connect(lambda: self.force_remove_item(item))

        # Lock button uses 🔒 / 🔓 (force emoji font on Windows)
        lock_btn = QtWidgets.QToolButton()
        lock_btn.setCheckable(True)
        lock_btn.setChecked(item.is_pinned)  # locked
        lock_btn.setFixedSize(26, 26)
        lock_btn.setStyleSheet("""
            QToolButton {
                border: 1px solid rgba(255,255,255,70);
                background: rgba(0,0,0,15);
                border-radius: 6px;
                padding: 0px;
            }
            QToolButton:hover {
                background: rgba(255,255,255,35);
            }
        """)

        # Ensure emoji is visible
        f = lock_btn.font()
        f.setFamily("Segoe UI Emoji")
        f.setPointSize(10)
        lock_btn.setFont(f)

        def refresh_lock():
            if lock_btn.isChecked():
                lock_btn.setText("🔒")
                lock_btn.setToolTip("Locked (won't auto-remove)")
            else:
                lock_btn.setText("🔓")
                lock_btn.setToolTip("Unlocked")

        def on_lock(checked: bool) -> None:
            item.is_pinned = checked
            refresh_lock()

        lock_btn.toggled.connect(on_lock)
        refresh_lock()

        btn_col.addWidget(remove_btn, 0, QtCore.Qt.AlignRight | QtCore.Qt.AlignTop)
        btn_col.addWidget(lock_btn, 0, QtCore.Qt.AlignRight)
        btn_col.addStretch(1)

        btn_wrap = QtWidgets.QWidget()
        btn_wrap.setLayout(btn_col)
        btn_wrap.setFixedWidth(32)

        grid.addWidget(btn_wrap, 0, 2, 2, 1, QtCore.Qt.AlignRight)

        grid.setColumnStretch(1, 1)
        return w

    # -------- remove / clear --------
    def remove_item(self, station_item: StationItem) -> None:
        """Normal remove: respects lock (locked can't be removed automatically)."""
        if station_item.is_pinned:
            return
        self.force_remove_item(station_item)

    def force_remove_item(self, station_item: StationItem) -> None:
        """Manual remove via X button: removes even if locked."""
        self.items = [x for x in self.items if x.id != station_item.id]
        for i in range(self.list.count()):
            it = self.list.item(i)
            s = it.data(QtCore.Qt.UserRole)
            if s and s.id == station_item.id:
                self.list.takeItem(i)
                break

        if self.list.count() == 0:
            self.hide_soft()

    def clear_unlocked(self) -> None:
        to_remove = []
        for i in range(self.list.count()):
            it = self.list.item(i)
            s = it.data(QtCore.Qt.UserRole)
            if s and not s.is_pinned:
                to_remove.append(s)
        for s in to_remove:
            self.force_remove_item(s)

    # -------- context menu --------
    def _show_context_menu(self, pos: QtCore.QPoint) -> None:
        it = self.list.itemAt(pos)
        if not it:
            return
        s: StationItem = it.data(QtCore.Qt.UserRole)

        menu = QtWidgets.QMenu(self)
        a_open_loc = menu.addAction("Open file location")
        a_copy_path = menu.addAction("Copy file path")
        menu.addSeparator()
        a_remove = menu.addAction("Remove (Unlocked only)")
        a_force = menu.addAction("Force remove")

        action = menu.exec(self.list.mapToGlobal(pos))
        if action == a_open_loc:
            open_in_explorer_select(s.path)
        elif action == a_copy_path:
            QtGui.QGuiApplication.clipboard().setText(s.path)
        elif action == a_remove:
            if not s.is_pinned:
                self.force_remove_item(s)
        elif action == a_force:
            self.force_remove_item(s)

    # -------- clipboard / preview --------
    def import_from_clipboard(self) -> None:
        cb = QtGui.QGuiApplication.clipboard()
        mime = cb.mimeData()

        if mime and mime.hasUrls():
            for url in mime.urls():
                if url.isLocalFile():
                    self.add_file(url.toLocalFile())
            self.show_soft()
            return

        img = cb.image()
        if not img.isNull():
            self.add_temp_image(img)
            self.show_soft()
            return

        txt = cb.text()
        if txt and txt.strip():
            self.add_temp_text(txt)
            self.show_soft()

    def export_selection_to_clipboard(self) -> None:
        selected = self.list.selectedItems()
        if not selected:
            return
        paths = []
        for it in selected:
            s: StationItem = it.data(QtCore.Qt.UserRole)
            if s and os.path.exists(s.path):
                paths.append(s.path)
        if not paths:
            return
        mime = QtCore.QMimeData()
        mime.setUrls([QtCore.QUrl.fromLocalFile(p) for p in paths])
        QtGui.QGuiApplication.clipboard().setMimeData(mime)

    def preview_selected(self) -> None:
        selected = self.list.selectedItems()
        if not selected:
            return
        s: StationItem = selected[0].data(QtCore.Qt.UserRole)
        if s:
            open_with_default_app(s.path)
