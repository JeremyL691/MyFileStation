from PySide6 import QtWidgets
from .settings import AppSettings, SettingsService
from .utils import create_app_icon, get_autostart_command, set_autostart_windows


class TrayController:
    def __init__(self, shelf, sensor, settings: AppSettings, settings_service: SettingsService) -> None:
        self.shelf = shelf
        self.sensor = sensor
        self.settings = settings
        self.settings_service = settings_service

        icon = create_app_icon()

        self.tray = QtWidgets.QSystemTrayIcon(icon)
        self.tray.setToolTip("MyFileStation")

        menu = QtWidgets.QMenu()

        act_show = menu.addAction("Show Shelf")
        act_hide = menu.addAction("Hide Shelf")
        menu.addSeparator()

        self.act_left = menu.addAction("Dock: Left")
        self.act_right = menu.addAction("Dock: Right")
        self.act_left.setCheckable(True)
        self.act_right.setCheckable(True)
        self.act_left.setChecked(self.settings.dock_side == "left")
        self.act_right.setChecked(self.settings.dock_side == "right")
        menu.addSeparator()

        self.act_remove_after_drag = menu.addAction("Auto-remove After Drag Out")
        self.act_remove_after_drag.setCheckable(True)
        self.act_remove_after_drag.setChecked(self.settings.remove_after_drag_out)

        self.act_cleanup_temp_on_exit = menu.addAction("Clean Up Temp Items On Exit")
        self.act_cleanup_temp_on_exit.setCheckable(True)
        self.act_cleanup_temp_on_exit.setChecked(self.settings.cleanup_temp_on_exit)

        self.act_autostart = menu.addAction("Auto-start with Windows")
        self.act_autostart.setCheckable(True)
        self.act_autostart.setChecked(self.settings.autostart)

        menu.addSeparator()
        act_exit = menu.addAction("Exit")

        act_show.triggered.connect(self.shelf.show_soft)
        act_hide.triggered.connect(self.shelf.hide_soft)

        self.act_left.triggered.connect(lambda: self._set_dock("left"))
        self.act_right.triggered.connect(lambda: self._set_dock("right"))

        self.act_remove_after_drag.toggled.connect(self._toggle_remove_after_drag)
        self.act_cleanup_temp_on_exit.toggled.connect(self._toggle_cleanup_temp_on_exit)
        self.act_autostart.toggled.connect(self._toggle_autostart)
        act_exit.triggered.connect(QtWidgets.QApplication.quit)
        self.tray.activated.connect(self._on_tray_activated)

        self.tray.setContextMenu(menu)
        self.tray.show()

        # Visible notification
        self.tray.showMessage(
            "MyFileStation",
            "Running in background. Right-click tray icon to open.",
            QtWidgets.QSystemTrayIcon.Information,
            2500,
        )

    def _on_tray_activated(self, reason: QtWidgets.QSystemTrayIcon.ActivationReason) -> None:
        if reason == QtWidgets.QSystemTrayIcon.Trigger:
            if self.shelf.isVisible():
                self.shelf.hide_soft()
            else:
                self.shelf.show_soft()

    def _set_dock(self, side: str) -> None:
        previous_side = self.settings.dock_side
        try:
            self.settings.dock_side = side
            self.settings_service.save(self.settings)
            self.act_left.setChecked(side == "left")
            self.act_right.setChecked(side == "right")
            self.shelf.reposition()
            self.sensor.reposition()
        except Exception as exc:
            self.settings.dock_side = previous_side
            self._set_checked(self.act_left, previous_side == "left")
            self._set_checked(self.act_right, previous_side == "right")
            self._show_warning("Dock Setting Failed", str(exc))

    def _toggle_remove_after_drag(self, enabled: bool) -> None:
        previous = self.settings.remove_after_drag_out
        try:
            self.settings.remove_after_drag_out = enabled
            self.settings_service.save(self.settings)
            self.shelf.refresh_status_text()
        except Exception as exc:
            self.settings.remove_after_drag_out = previous
            self._set_checked(self.act_remove_after_drag, previous)
            self._show_warning("Setting Failed", str(exc))

    def _toggle_cleanup_temp_on_exit(self, enabled: bool) -> None:
        previous = self.settings.cleanup_temp_on_exit
        try:
            self.settings.cleanup_temp_on_exit = enabled
            self.settings_service.save(self.settings)
        except Exception as exc:
            self.settings.cleanup_temp_on_exit = previous
            self._set_checked(self.act_cleanup_temp_on_exit, previous)
            self._show_warning("Setting Failed", str(exc))

    def _toggle_autostart(self, enabled: bool) -> None:
        previous = self.settings.autostart
        try:
            self.settings.autostart = enabled
            self.settings_service.save(self.settings)

            cmd = get_autostart_command()
            set_autostart_windows(enabled, "MyFileStation", cmd)
        except Exception as exc:
            self.settings.autostart = previous
            self._set_checked(self.act_autostart, previous)
            self._show_warning("Auto-start Failed", str(exc))

    def _set_checked(self, action: QtWidgets.QAction, checked: bool) -> None:
        action.blockSignals(True)
        action.setChecked(checked)
        action.blockSignals(False)

    def _show_warning(self, title: str, message: str) -> None:
        QtWidgets.QMessageBox.warning(None, title, message)
