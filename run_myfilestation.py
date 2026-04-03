"""
Bootstrap script for development and packaging.

This file makes the local `src/` layout runnable without requiring the package
to be installed first.
"""

from pathlib import Path
import sys
import ctypes


ROOT = Path(__file__).resolve().parent
SRC = ROOT / "src"

if str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))

try:
    from myfilestation.main import main
except ModuleNotFoundError as exc:
    message = (
        "MyFileStation could not start because a required dependency is missing.\n\n"
        f"Missing module: {exc.name}\n\n"
        "Run:\n"
        "python -m pip install -r requirements.txt"
    )
    try:
        ctypes.windll.user32.MessageBoxW(None, message, "MyFileStation startup failed", 0x10)
    except Exception:
        print(message, file=sys.stderr)
    raise


if __name__ == "__main__":
    main()
