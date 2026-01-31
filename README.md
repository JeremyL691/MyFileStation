# MyFileStation

MyFileStation is a lightweight “file shelf” for Windows. It stays hidden most of the time, then appears when you drag files toward the screen edge. Drop items in, keep them temporarily, copy/drag them out later, lock frequently reused files, and remove items quickly.

The goal is to make temporary file staging feel effortless—especially when you’re moving files between folders, apps, browsers, or chat tools.

---

## Why this exists

Sometimes you just need a small holding area:

- collecting files from different folders before sending them somewhere
- moving multiple assets between apps (design tools, browsers, messaging apps)
- keeping a few always-used files ready to drag out at any time

MyFileStation is designed to stay out of the way and be available only when you’re already doing a drag action.

---

## Key features

### Edge-triggered shelf (auto show / auto hide)
- Runs quietly in the background.
- When you drag a file toward the configured screen edge (left or right), the shelf appears.
- If you cancel the drag (release the mouse without dropping) and the shelf is empty, the window fades out and hides.

### Drag & drop in/out (single or multiple)
- Drop files into the shelf from File Explorer or other apps.
- Select one or multiple items and drag them out to a folder or another application.

### Clipboard workflow
- **Ctrl + V (Import):**
  - If the clipboard contains files copied in Explorer, those files are added.
  - If the clipboard contains an image/screenshot, it’s stored as a temporary image file.
  - If the clipboard contains text, it’s stored as a temporary `.txt` file.
- **Ctrl + C (Export):**
  - Copies selected shelf items to clipboard as file URLs, so you can paste into apps/folders that accept files.

### Visual list (thumbnail + name + path)
- Image files display real thumbnails.
- Each entry shows a readable file name and a file path (paths are elided to fit the UI cleanly).

### Lock / unlock

- Click the lock button on an item to toggle **Locked / Unlocked**.
- Locked items remain on the shelf even after you drag them out.

This is useful for always-needed files like templates or frequently reused assets.

### Fast remove
- Each item has a small **×** button to remove it instantly (manual remove always works, even for locked items).

---

## Screenshots / demo (recommended)

Add images here once you have them:

- `docs/screenshot-main.png` (main shelf)
- `docs/screenshot-locked.png` (locked item + remove button)
- `docs/demo.gif` (short workflow demo)

Tip: A single GIF showing “drag to edge → shelf appears → drop → lock → drag out” goes a long way.

---

## Project structure

```text
MyFileStation/
  src/
    myfilestation/
      main.py
      shelf_window.py
      edge_sensor.py
      tray.py
      settings.py
      models.py
      utils.py
  .gitignore
  LICENSE
  README.md
```

High-level responsibilities:
- `edge_sensor.py`: detects drag-to-edge behavior
- `shelf_window.py`: shelf UI + item interactions (lock/remove/drag out)
- `tray.py`: system tray icon + quick actions
- `settings.py`: basic preferences (dock side, behavior toggles)
- `models.py`: data model for shelf items
- `utils.py`: helper functions (temp files, thumbnails, open in explorer, etc.)

---

## Requirements

- Windows 10/11
- Python 3.10+ (3.11+ recommended)
- Dependencies installed via `pip` (see `requirements.txt`)

---

## Quick start (run locally)

### 1) Clone the repository
```bash
git clone https://github.com/JeremyL691/MyFileStation.git
cd MyFileStation
```

### 2) Create and activate a virtual environment (PowerShell)
```powershell
python -m venv .venv
.\.venv\Scripts\Activate.ps1
```

If PowerShell blocks activation, run this once (then try again):
```powershell
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
```

### 3) Install dependencies
```powershell
pip install -r requirements.txt
```

### 4) Run the app
This project uses a `src/` layout, so run from the repo root:
```powershell
$env:PYTHONPATH="src"
python -m myfilestation.main
```

The app should start in the background (typically with a tray icon). Then try dragging a file toward the configured edge.

---

## Notes about drag & drop on Windows

If clipboard import works but drag-and-drop does not, it is often a privilege/UAC issue:
- If MyFileStation is running as Administrator but File Explorer is not, Windows may block drag-and-drop.
- Recommendation: run MyFileStation as a normal user (non-admin) for daily usage.

---

## Roadmap (ideas)

- Preferences UI (dock side, remove behavior, autostart)
- Better preview support (quick view panel / system preview integration)
- More robust “file drag vs window drag” distinction
- Improved batch export UX (dragging multiple items as a stack)
- Polished icons and file type indicators for non-image files

---

## Contributing

Issues and PRs are welcome.

When filing a bug, please include:
- Windows version
- steps to reproduce
- expected vs actual behavior
- screenshots / short recording if UI-related

PR guidelines:
- keep changes focused and easy to review
- explain the reason for the change
- add screenshots for UI changes when possible

---

## License

MIT License. See `LICENSE`.
