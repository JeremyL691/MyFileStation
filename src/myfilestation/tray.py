from PySide6 import QtWidgets
from .settings import AppSettings, SettingsService
from .utils import set_autostart_windows, get_running_python_exe_for_autostart


class TrayController:
    def __init__(self, shelf, sensor, settings: AppSettings, settings_service: SettingsService) -> None:
        self.shelf = shelf
        self.sensor = sensor
        self.settings = settings
        self.settings_service = settings_service

        # Use a standard icon so tray ALWAYS appears
        icon = QtWidgets.QApplication.style().standardIcon(QtWidgets.QStyle.SP_DirIcon)

        self.tray = QtWidgets.QSystemTrayIcon(icon)
        self.tray.setToolTip("MyFileStation")

        menu = QtWidgets.QMenu()

        act_show = menu.addAction("Show Shelf")
        act_hide = menu.addAction("Hide Shelf")
        menu.addSeparator()

        act_left = menu.addAction("Dock: Left")
        act_right = menu.addAction("Dock: Right")
        menu.addSeparator()

        self.act_autostart = menu.addAction("Auto-start with Windows")
        self.act_autostart.setCheckable(True)
        self.act_autostart.setChecked(self.settings.autostart)

        menu.addSeparator()
        act_exit = menu.addAction("Exit")

        act_show.triggered.connect(self.shelf.show_soft)
        act_hide.triggered.connect(self.shelf.hide_soft)

        act_left.triggered.connect(lambda: self._set_dock("left"))
        act_right.triggered.connect(lambda: self._set_dock("right"))

        self.act_autostart.toggled.connect(self._toggle_autostart)
        act_exit.triggered.connect(QtWidgets.QApplication.quit)

        self.tray.setContextMenu(menu)
        self.tray.show()

        # Visible notification
        self.tray.showMessage(
            "MyFileStation",
            "Running in background. Right-click tray icon to open.",
            QtWidgets.QSystemTrayIcon.Information,
            2500,
        )

    def _set_dock(self, side: str) -> None:
        self.settings.dock_side = side
        self.settings_service.save(self.settings)
        self.shelf.reposition()
        self.sensor.reposition()

    def _toggle_autostart(self, enabled: bool) -> None:
        self.settings.autostart = enabled
        self.settings_service.save(self.settings)

        py = get_running_python_exe_for_autostart()
        cmd = f'"{py}" -m myfilestation.main'
        set_autostart_windows(enabled, "MyFileStation", cmd)
