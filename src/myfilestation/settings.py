import json
import os
from dataclasses import dataclass, asdict


def get_appdata_dir() -> str:
    # Use roaming AppData for settings
    base = os.environ.get("APPDATA", os.path.expanduser("~"))
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


class SettingsService:
    def __init__(self) -> None:
        self._path = os.path.join(get_appdata_dir(), "settings.json")

    def load(self) -> AppSettings:
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
            )
        except Exception:
            # Fall back to defaults if config is broken
            return AppSettings()

    def save(self, settings: AppSettings) -> None:
        with open(self._path, "w", encoding="utf-8") as f:
            json.dump(asdict(settings), f, indent=2)
