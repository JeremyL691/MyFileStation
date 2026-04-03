# MyFileStation

MyFileStation is a lightweight Windows shelf for temporary file hand-offs.

Instead of minimizing a stack of windows just to drop a file somewhere else, you can keep a narrow shelf on the left or right side of the screen:

1. Drag a file from Explorer or the Desktop toward the configured screen edge.
2. The shelf slides out.
3. Drop the file onto the shelf.
4. Open your target app and drag the file back out when you need it.

It also accepts clipboard text and images with `Ctrl+V`.

## Current MVP Capabilities

- Edge reveal for local file drags from `Explorer` and the `Desktop`
- Drop files into the shelf
- Paste files, text, and images from the clipboard
- Drag files back out to other apps
- Pin items so they are protected from auto-remove and bulk clear
- Tray controls for dock side, auto-remove, and auto-start
- Native Windows file icons plus thumbnails for image files

## Important Limits

- Edge reveal currently watches `Explorer` and `Desktop` file views only
- Browser-originated drags are not yet supported
- This project targets `Windows 10/11`
- Running the app as `Administrator` disables Explorer drag and drop because of Windows integrity rules

## Project Layout

```text
run_myfilestation.py        Development entry point
requirements.txt            Runtime dependencies
src/myfilestation/          Application package
```

## Development Setup

```powershell
python -m pip install -r requirements.txt
python run_myfilestation.py
```

The bootstrap script adds `src/` to `sys.path`, so you do not need to install the package in editable mode just to run it locally.

## Build A Windows EXE

```powershell
python -m pip install -r requirements-packaging.txt
python -m PyInstaller --noconfirm --clean MyFileStation.spec
```

The packaged executable will be written to `dist/MyFileStation.exe`.

## Verification

```powershell
python -m pytest -q
python -m pytest -m smoke
```

A full Windows manual regression checklist is available in [TESTING.md](./TESTING.md).

## Controls

- `Ctrl+V`: import files, text, or images from the clipboard
- `Ctrl+C`: copy selected shelf files to the clipboard as file paths
- `Ctrl+A`: select all items
- `Enter` or `Space`: open the selected item
- `Delete`: remove selected unpinned items
- `Esc`: hide the shelf
- Double-click an item: open it
- Right-click an item: open, reveal in Explorer, copy path, pin/unpin, remove

## Auto-Start Behavior

- In development, auto-start points to `pythonw.exe` plus `run_myfilestation.py`
- In a packaged build, auto-start should point directly to the built executable

## Packaging Notes

`run_myfilestation.py` exists so the local `src/` package layout works in development and when bundled. Do not point packaging directly at `src/myfilestation/main.py`.

## License

MIT
