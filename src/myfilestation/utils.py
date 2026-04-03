import os
import time
import subprocess
from pathlib import Path
import sys
from typing import Iterable, List
import uuid

from PySide6 import QtCore, QtGui, QtWidgets


def get_temp_dir() -> str:
    try:
        path = os.path.join(os.environ.get("TEMP", str(Path.home() / "AppData" / "Local" / "Temp")), "MyFileStation")
        os.makedirs(path, exist_ok=True)
        return path
    except Exception as exc:
        raise RuntimeError(f"Temporary storage could not be prepared.\n\n{exc}") from exc


def is_image_file(path: str) -> bool:
    ext = os.path.splitext(path)[1].lower()
    return ext in [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"]


def create_temp_text_file(text: str) -> str:
    try:
        d = get_temp_dir()
        name = f"mfs_{time.strftime('%Y%m%d_%H%M%S')}_{uuid.uuid4().hex[:8]}.txt"
        p = os.path.join(d, name)
        with open(p, "w", encoding="utf-8") as f:
            f.write(text)
        return p
    except Exception as exc:
        raise RuntimeError(f"Clipboard text could not be saved to a temporary file.\n\n{exc}") from exc


def create_temp_image_file_from_qimage(qimage) -> str:
    try:
        d = get_temp_dir()
        name = f"mfs_{time.strftime('%Y%m%d_%H%M%S')}_{uuid.uuid4().hex[:8]}.png"
        p = os.path.join(d, name)
        if not qimage.save(p, "PNG"):
            raise RuntimeError("Qt could not encode the image as PNG.")
        return p
    except Exception as exc:
        raise RuntimeError(f"Clipboard image could not be saved to a temporary file.\n\n{exc}") from exc


def delete_file_if_exists(path: str) -> None:
    try:
        if path and os.path.exists(path):
            os.remove(path)
    except FileNotFoundError:
        return


def first_missing_paths(paths: Iterable[str]) -> List[str]:
    missing = []
    for path in paths:
        if not os.path.exists(path):
            missing.append(path)
    return missing


def open_with_default_app(path: str) -> None:
    if not os.path.exists(path):
        raise FileNotFoundError(path)

    try:
        os.startfile(path)  # type: ignore[attr-defined]
    except Exception as exc:
        raise RuntimeError(f"The file could not be opened with the default application.\n\n{exc}") from exc


def open_in_explorer_select(path: str) -> None:
    if not os.path.exists(path):
        raise FileNotFoundError(path)
    try:
        subprocess.Popen(["explorer.exe", f'/select,"{path}"'])
    except Exception as exc:
        raise RuntimeError(f"Explorer could not reveal the file.\n\n{exc}") from exc


def set_autostart_windows(enable: bool, app_name: str, exe_path: str) -> None:
    """
    Use HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run.
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
    """Use pythonw in development to avoid opening a console window."""
    # pythonw.exe usually exists next to python.exe
    py = Path(sys.executable)
    pyw = py.with_name("pythonw.exe")
    return str(pyw if pyw.exists() else py)


def is_frozen_app() -> bool:
    return bool(getattr(sys, "frozen", False))


def get_launch_target() -> str:
    if is_frozen_app():
        return str(Path(sys.executable).resolve())
    return str(Path(__file__).resolve().parents[2] / "run_myfilestation.py")


def get_autostart_command() -> str:
    target = get_launch_target()
    if is_frozen_app():
        return f'"{target}"'
    py = get_running_python_exe_for_autostart()
    return f'"{py}" "{target}"'


def create_app_icon(size: int = 64) -> QtGui.QIcon:
    pixmap = QtGui.QPixmap(size, size)
    pixmap.fill(QtCore.Qt.transparent)

    painter = QtGui.QPainter(pixmap)
    painter.setRenderHint(QtGui.QPainter.Antialiasing, True)

    base_rect = QtCore.QRectF(4, 4, size - 8, size - 8)
    base_color = QtGui.QColor("#1F2835")
    accent = QtGui.QColor("#7AA7FF")
    edge = QtGui.QColor("#AFC7FF")

    painter.setPen(QtGui.QPen(QtGui.QColor("#10151D"), max(1, size // 18)))
    painter.setBrush(base_color)
    painter.drawRoundedRect(base_rect, size * 0.18, size * 0.18)

    shelf_rect = QtCore.QRectF(size * 0.22, size * 0.28, size * 0.50, size * 0.44)
    painter.setPen(QtGui.QPen(edge, max(2, size // 14)))
    painter.setBrush(QtCore.Qt.NoBrush)
    painter.drawRoundedRect(shelf_rect, size * 0.10, size * 0.10)

    painter.setPen(QtCore.Qt.NoPen)
    painter.setBrush(accent)
    painter.drawRoundedRect(QtCore.QRectF(size * 0.72, size * 0.22, size * 0.10, size * 0.56), 4, 4)
    painter.drawRoundedRect(QtCore.QRectF(size * 0.28, size * 0.47, size * 0.48, size * 0.10), 4, 4)

    painter.end()
    return QtGui.QIcon(pixmap)


def create_symbol_icon(symbol: str, size: int = 18, accent: str = "#DDE7FF") -> QtGui.QIcon:
    pixmap = QtGui.QPixmap(size, size)
    pixmap.fill(QtCore.Qt.transparent)

    painter = QtGui.QPainter(pixmap)
    painter.setRenderHint(QtGui.QPainter.Antialiasing, True)
    font = QtGui.QFont("Segoe Fluent Icons")
    if not QtGui.QFontInfo(font).family().lower().startswith("segoe fluent"):
        font = QtGui.QFont("Segoe MDL2 Assets")
    font.setPixelSize(max(12, size - 2))
    painter.setFont(font)
    painter.setPen(QtGui.QColor(accent))
    painter.drawText(QtCore.QRectF(0, 0, size, size), QtCore.Qt.AlignCenter, symbol)
    painter.end()
    return QtGui.QIcon(pixmap)


def create_line_icon(kind: str, size: int = 18, color: str = "#DDE7FF") -> QtGui.QIcon:
    pixmap = QtGui.QPixmap(size, size)
    pixmap.fill(QtCore.Qt.transparent)

    painter = QtGui.QPainter(pixmap)
    painter.setRenderHint(QtGui.QPainter.Antialiasing, True)
    pen = QtGui.QPen(QtGui.QColor(color), max(1.6, size / 10.0))
    pen.setCapStyle(QtCore.Qt.RoundCap)
    pen.setJoinStyle(QtCore.Qt.RoundJoin)
    painter.setPen(pen)
    painter.setBrush(QtCore.Qt.NoBrush)

    if kind == "close":
        painter.drawLine(size * 0.28, size * 0.28, size * 0.72, size * 0.72)
        painter.drawLine(size * 0.72, size * 0.28, size * 0.28, size * 0.72)
    elif kind == "pin":
        path = QtGui.QPainterPath()
        path.moveTo(size * 0.36, size * 0.20)
        path.lineTo(size * 0.64, size * 0.20)
        path.lineTo(size * 0.60, size * 0.52)
        path.lineTo(size * 0.50, size * 0.62)
        path.lineTo(size * 0.40, size * 0.52)
        path.closeSubpath()
        painter.drawPath(path)
        painter.drawLine(size * 0.50, size * 0.62, size * 0.50, size * 0.84)
    elif kind == "pin-filled":
        fill = QtGui.QColor(color)
        fill.setAlpha(70)
        path = QtGui.QPainterPath()
        path.moveTo(size * 0.36, size * 0.20)
        path.lineTo(size * 0.64, size * 0.20)
        path.lineTo(size * 0.60, size * 0.52)
        path.lineTo(size * 0.50, size * 0.62)
        path.lineTo(size * 0.40, size * 0.52)
        path.closeSubpath()
        painter.fillPath(path, fill)
        painter.drawPath(path)
        painter.drawLine(size * 0.50, size * 0.62, size * 0.50, size * 0.84)
    elif kind == "text":
        painter.drawRoundedRect(QtCore.QRectF(size * 0.18, size * 0.14, size * 0.64, size * 0.72), 3, 3)
        painter.drawLine(size * 0.30, size * 0.38, size * 0.70, size * 0.38)
        painter.drawLine(size * 0.30, size * 0.52, size * 0.66, size * 0.52)
        painter.drawLine(size * 0.30, size * 0.66, size * 0.58, size * 0.66)
    elif kind == "image":
        painter.drawRoundedRect(QtCore.QRectF(size * 0.14, size * 0.18, size * 0.72, size * 0.62), 3, 3)
        painter.drawEllipse(QtCore.QPointF(size * 0.34, size * 0.36), size * 0.06, size * 0.06)
        mountain = QtGui.QPainterPath()
        mountain.moveTo(size * 0.24, size * 0.68)
        mountain.lineTo(size * 0.44, size * 0.48)
        mountain.lineTo(size * 0.56, size * 0.60)
        mountain.lineTo(size * 0.70, size * 0.42)
        mountain.lineTo(size * 0.76, size * 0.68)
        painter.drawPath(mountain)
    elif kind == "folder":
        painter.drawRoundedRect(QtCore.QRectF(size * 0.14, size * 0.30, size * 0.72, size * 0.44), 3, 3)
        painter.drawLine(size * 0.20, size * 0.30, size * 0.36, size * 0.20)
        painter.drawLine(size * 0.36, size * 0.20, size * 0.52, size * 0.20)
    painter.end()
    return QtGui.QIcon(pixmap)


def get_file_icon(path: str) -> QtGui.QIcon:
    provider = QtWidgets.QFileIconProvider()
    if path and os.path.exists(path):
        info = QtCore.QFileInfo(path)
        return provider.icon(info)
    return provider.icon(QtWidgets.QFileIconProvider.File)
