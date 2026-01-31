import time
from PySide6 import QtCore, QtGui

import win32api
import win32con
import win32gui


class EdgeSensorWindow(QtCore.QObject):
    """
    Stable edge trigger (no dragEnter/OLE needed).

    Triggers when:
      - Left mouse button is down
      - Mouse moved enough (drag)
      - Cursor is over an Explorer/desktop file view (avoid dragging app windows)
      - Cursor near the configured dock edge
    """

    supported_drag_detected = QtCore.Signal(object)  # emits None

    def __init__(self, settings):
        super().__init__()
        self.settings = settings

        self.edge_threshold = 48   # px near screen edge
        self.drag_dist = 12        # pixels moved to treat as drag
        self.drag_delay_ms = 60    # held time before treat as drag

        self._active = True
        self._dragging = False
        self._triggered = False
        self._down_pos = None
        self._down_t = 0.0

        self._timer = QtCore.QTimer(self)
        self._timer.setInterval(16)  # ~60fps
        self._timer.timeout.connect(self._tick)
        self._timer.start()

    # kept for compatibility with TrayController
    def suspend(self) -> None:
        self._active = False

    def resume(self) -> None:
        self._active = True

    def reposition(self) -> None:
        return

    def _near_edge(self, gpos: QtCore.QPoint) -> bool:
        screen = QtGui.QGuiApplication.screenAt(gpos)
        if not screen:
            screen = QtGui.QGuiApplication.primaryScreen()
        if not screen:
            return False

        r = screen.availableGeometry()
        x = gpos.x()

        if self.settings.dock_side == "left":
            return x <= (r.x() + self.edge_threshold)
        else:
            return x >= (r.x() + r.width() - self.edge_threshold)

    def _window_class_chain(self, hwnd: int, max_depth: int = 10):
        chain = []
        cur = hwnd
        for _ in range(max_depth):
            if not cur:
                break
            try:
                chain.append(win32gui.GetClassName(cur))
            except Exception:
                break
            cur = win32gui.GetParent(cur)
        return chain

    def _is_file_view_under_cursor(self) -> bool:
        """
        Works for BOTH:
          - File Explorer window
          - Desktop icons (Progman/WorkerW + SHELLDLL_DefView)
        """
        x, y = win32gui.GetCursorPos()
        hwnd = win32gui.WindowFromPoint((x, y))
        if not hwnd:
            return False

        chain = self._window_class_chain(hwnd)
        top = win32gui.GetAncestor(hwnd, win32con.GA_ROOT)
        try:
            top_cls = win32gui.GetClassName(top)
        except Exception:
            top_cls = ""

        view_classes = {"DirectUIHWND", "SysListView32", "SHELLDLL_DefView"}
        explorer_frames = {"CabinetWClass", "ExploreWClass"}
        desktop_frames = {"Progman", "WorkerW"}

        has_view = any(c in view_classes for c in chain)
        is_explorer = top_cls in explorer_frames
        is_desktop = top_cls in desktop_frames

        return has_view and (is_explorer or is_desktop)

    def _tick(self) -> None:
        if not self._active:
            return

        ldown = (win32api.GetAsyncKeyState(win32con.VK_LBUTTON) & 0x8000) != 0
        gpos = QtGui.QCursor.pos()

        if ldown and self._down_pos is None:
            self._down_pos = gpos
            self._down_t = time.time()
            self._dragging = False
            self._triggered = False
            return

        if (not ldown) and self._down_pos is not None:
            self._down_pos = None
            self._dragging = False
            self._triggered = False
            return

        if not ldown or self._down_pos is None:
            return

        dx = abs(gpos.x() - self._down_pos.x())
        dy = abs(gpos.y() - self._down_pos.y())
        moved = (dx + dy) >= self.drag_dist
        held_ms = (time.time() - self._down_t) * 1000.0

        if (not self._dragging) and moved and (held_ms >= self.drag_delay_ms):
            self._dragging = True

        if not self._dragging:
            return

        if not self._is_file_view_under_cursor():
            return

        if (not self._triggered) and self._near_edge(gpos):
            self._triggered = True
            self.supported_drag_detected.emit(None)
