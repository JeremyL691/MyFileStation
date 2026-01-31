"""
Entry script for packaging.

Do NOT run src/myfilestation/main.py directly with PyInstaller because it uses
relative imports (e.g. "from .foo import bar"). Those only work when the module
is imported as part of the 'myfilestation' package.

This wrapper imports the package module properly.
"""

from myfilestation.main import main


if __name__ == "__main__":
    main()
