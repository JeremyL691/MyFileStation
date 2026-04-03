# MyFileStation

MyFileStation is a small Windows shelf for temporary file hand-offs.

You drag a file from Explorer or the Desktop to the screen edge, the shelf slides out, you drop the file there, then drag it back out into the app you actually want to use.

## What It Does

- Reveals from the left or right screen edge
- Accepts local files from Explorer/Desktop
- Accepts clipboard files, text, and images with `Ctrl+V`
- Lets you drag items back out to other apps
- Supports pinning so important items are not auto-removed
- Runs from the system tray with dock side, auto-remove, cleanup, and auto-start settings

## Current Limits

- Edge reveal only watches `Explorer` and `Desktop` drags
- Browser-originated drags are not supported yet
- Windows only
- Running as Administrator disables Explorer drag-and-drop

## Run From Source

```powershell
python -m pip install -r requirements.txt
python run_myfilestation.py
```

## Build The EXE

```powershell
python -m pip install -r requirements-packaging.txt
python -m PyInstaller --noconfirm --clean MyFileStation.spec
```

Output:

```text
dist/MyFileStation.exe
```

## Verify

```powershell
python -m pytest -q
python -m pytest -m smoke
```

Manual Windows regression steps are in [TESTING.md](./TESTING.md).

## Controls

- `Ctrl+V` import from clipboard
- `Ctrl+C` copy selected file entries to clipboard
- `Ctrl+A` select all
- `Enter` or `Space` open selected item
- `Delete` remove selected unpinned items
- `Esc` hide the shelf
- Double-click open
- Right-click for open, reveal, copy path, pin, and remove actions

## Notes

- Development auto-start uses `pythonw.exe` plus `run_myfilestation.py`
- Packaged auto-start uses the built executable path
- `run_myfilestation.py` is the correct entry point for both local development and packaging

## License

MIT
