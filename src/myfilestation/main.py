import sys
import traceback
import ctypes
from PySide6 import QtWidgets

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

        if is_running_as_admin():
            QtWidgets.QMessageBox.critical(
                None,
                "MyFileStation - Drag & Drop disabled",
                "You are running as Administrator.\n\n"
                "Windows blocks Explorer drag & drop into elevated apps.\n"
                "Please run VSCode/app without Administrator."
            )
            return

        settings_service = SettingsService()
        settings = settings_service.load()

        shelf = ShelfWindow(settings)
        sensor = EdgeSensorWindow(settings)

        def on_edge_drag(_):
            shelf.show_from_edge_drag()


        sensor.supported_drag_detected.connect(on_edge_drag)

        TrayController(shelf, sensor, settings, settings_service)

        sys.exit(app.exec())

    except Exception:
        err = traceback.format_exc()
        QtWidgets.QMessageBox.critical(None, "MyFileStation failed", err)
        raise


if __name__ == "__main__":
    main()
