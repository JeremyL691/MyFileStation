import json
import os
from dataclasses import dataclass, asdict
from typing import Optional


def get_appdata_dir(base_dir: Optional[str] = None) -> str:
    # Use roaming AppData for settings
    base = base_dir or os.environ.get("APPDATA", os.path.expanduser("~"))
    path = os.path.join(base, "MyFileStation")
    os.makedirs(path, exist_ok=True)
    return path


@dataclass
class AppSettings:
    # Dock side: "left" or "right"
    dock_side: str = "right"

    # If True, remove items after drag-out success
    remove_after_drag_out: bool = True

    # If True, start with Windows
    autostart: bool = False

    # If True, delete unpinned temporary items when the app exits
    cleanup_temp_on_exit: bool = True


class SettingsService:
    def __init__(self, path: Optional[str] = None, appdata_dir: Optional[str] = None) -> None:
        self._path = path or os.path.join(get_appdata_dir(appdata_dir), "settings.json")
        self.last_warning: Optional[str] = None

    def load(self) -> AppSettings:
        self.last_warning = None
        try:
            if not os.path.exists(self._path):
                s = AppSettings()
                self.save(s)
                return s

            with open(self._path, "r", encoding="utf-8") as f:
                data = json.load(f)

            return AppSettings(
                dock_side=data.get("dock_side", "right"),
                remove_after_drag_out=bool(data.get("remove_after_drag_out", True)),
                autostart=bool(data.get("autostart", False)),
                cleanup_temp_on_exit=bool(data.get("cleanup_temp_on_exit", True)),
            )
        except Exception as exc:
            # Fall back to defaults if config is broken
            self.last_warning = f"Settings file could not be read. Defaults were restored.\n\n{exc}"
            return AppSettings()

    def save(self, settings: AppSettings) -> None:
        with open(self._path, "w", encoding="utf-8") as f:
            json.dump(asdict(settings), f, indent=2)
