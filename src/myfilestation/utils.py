import os
import time
import subprocess
from typing import Optional
from pathlib import Path

from .settings import get_appdata_dir


def get_temp_dir() -> str:
    path = os.path.join(os.environ.get("TEMP", str(Path.home() / "AppData" / "Local" / "Temp")), "MyFileStation")
    os.makedirs(path, exist_ok=True)
    return path


def is_image_file(path: str) -> bool:
    ext = os.path.splitext(path)[1].lower()
    return ext in [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"]


def create_temp_text_file(text: str) -> str:
    d = get_temp_dir()
    name = f"mfs_{time.strftime('%Y%m%d_%H%M%S')}_{int(time.time() * 1000) % 1000}.txt"
    p = os.path.join(d, name)
    with open(p, "w", encoding="utf-8") as f:
        f.write(text)
    return p


def create_temp_image_file_from_qimage(qimage) -> str:
    # Save QImage as PNG
    d = get_temp_dir()
    name = f"mfs_{time.strftime('%Y%m%d_%H%M%S')}_{int(time.time() * 1000) % 1000}.png"
    p = os.path.join(d, name)
    qimage.save(p, "PNG")
    return p


def open_with_default_app(path: str) -> None:
    # Windows: os.startfile is easiest
    try:
        os.startfile(path)  # type: ignore[attr-defined]
    except Exception:
        # Fallback
        subprocess.Popen(["cmd", "/c", "start", "", path], shell=True)


def open_in_explorer_select(path: str) -> None:
    if not os.path.exists(path):
        return
    subprocess.Popen(["explorer.exe", f'/select,"{path}"'])


def copy_text_to_clipboard(text: str) -> None:
    # Qt clipboard is handled in GUI; keep this helper empty for now
    pass


def set_autostart_windows(enable: bool, app_name: str, exe_path: str) -> None:
    """
    Use HKCU\Software\Microsoft\Windows\CurrentVersion\Run
    No admin rights needed.
    """
    import winreg

    key_path = r"Software\Microsoft\Windows\CurrentVersion\Run"
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path, 0, winreg.KEY_SET_VALUE) as key:
        if enable:
            winreg.SetValueEx(key, app_name, 0, winreg.REG_SZ, exe_path)
        else:
            try:
                winreg.DeleteValue(key, app_name)
            except FileNotFoundError:
                pass


def get_running_python_exe_for_autostart() -> str:
    """
    During development, autostart should point to pythonw + -m myfilestation.main.
    For packaged app, you will use the built exe path.
    """
    # pythonw.exe usually exists next to python.exe
    py = Path(os.sys.executable)
    pyw = py.with_name("pythonw.exe")
    return str(pyw if pyw.exists() else py)
