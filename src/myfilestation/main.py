import sys
import traceback
import ctypes
from PySide6 import QtWidgets

from .controller import ShelfController
from .settings import SettingsService
from .shelf_window import ShelfWindow
from .edge_sensor import EdgeSensorWindow
from .tray import TrayController


def is_running_as_admin() -> bool:
    try:
        return bool(ctypes.windll.shell32.IsUserAnAdmin())
    except Exception:
        return False


def main() -> None:
    try:
        app = QtWidgets.QApplication(sys.argv)
        app.setApplicationName("MyFileStation")
        app.setQuitOnLastWindowClosed(False)

        if not QtWidgets.QSystemTrayIcon.isSystemTrayAvailable():
            QtWidgets.QMessageBox.critical(
                None,
                "MyFileStation - System Tray Unavailable",
                "A system tray is required to run MyFileStation.\n\n"
                "Make sure Explorer is running and the desktop shell is available.",
            )
            return

        if is_running_as_admin():
            QtWidgets.QMessageBox.critical(
                None,
                "MyFileStation - Drag & Drop disabled",
                "You are running as Administrator.\n\n"
                "Windows blocks Explorer drag & drop into elevated apps.\n"
                "Please run VSCode/app without Administrator."
            )
            return

        try:
            settings_service = SettingsService()
            settings = settings_service.load()
        except Exception as exc:
            QtWidgets.QMessageBox.critical(
                None,
                "MyFileStation - Settings Unavailable",
                f"Application settings could not be initialized.\n\n{exc}",
            )
            return

        controller = ShelfController(settings)

        shelf = ShelfWindow(settings, controller)
        sensor = EdgeSensorWindow(settings)

        def on_edge_drag(_):
            shelf.show_from_edge_drag()


        sensor.supported_drag_detected.connect(on_edge_drag)

        try:
            tray = TrayController(shelf, sensor, settings, settings_service)
        except Exception as exc:
            QtWidgets.QMessageBox.critical(
                None,
                "MyFileStation - Tray Initialization Failed",
                f"The system tray controller could not be started.\n\n{exc}",
            )
            return
        app.setWindowIcon(tray.tray.icon())
        app.aboutToQuit.connect(controller.cleanup_on_exit)

        if settings_service.last_warning:
            QtWidgets.QMessageBox.warning(
                None,
                "MyFileStation - Settings Reset",
                settings_service.last_warning,
            )

        sys.exit(app.exec())

    except Exception:
        err = traceback.format_exc()
        QtWidgets.QMessageBox.critical(None, "MyFileStation failed", err)
        raise


if __name__ == "__main__":
    main()
