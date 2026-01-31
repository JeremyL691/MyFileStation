# MyFileStation

MyFileStation is a lightweight “file shelf” for Windows. It stays hidden most of the time, then appears when you drag files toward the screen edge. Drop items into the shelf, keep them temporarily, copy them out later, lock frequently reused files, and remove things quickly.

The goal is to make “temporary file staging” feel effortless—especially when you’re moving files between folders, apps, chats, or browsers.

---

## Why this exists

Sometimes you just need a small holding area:
- you’re collecting files from different folders before sending them somewhere
- you’re moving multiple assets between apps (design tools, browser downloads, chat apps)
- you want a couple of “always-used” files ready to drag out whenever you need them

MyFileStation is designed to stay out of the way and be instantly available when you’re already doing a drag action.

---

## Features

### Edge-triggered shelf (auto show / auto hide)
- The app runs quietly in the background.
- When you **drag a file and move toward the configured screen edge** (left or right), the shelf appears.
- If you cancel the drag (release the mouse without dropping) and the shelf is empty, the window fades out and hides.

### Drag & Drop in/out (single or multiple)
- Drop files into the shelf from File Explorer or other apps.
- Select one or multiple items and drag them out to a folder or another application.

### Clipboard support
- **Ctrl + V (Import):**
  - If your clipboard contains files copied in Explorer, they will be added.
  - If your clipboard contains an image/screenshot, it will be stored as a temporary image file.
  - If your clipboard contains text, it will be stored as a temporary `.txt` file.
- **Ctrl + C (Export):**
  - Copies the selected shelf items to clipboard as file URLs, so you can paste into apps that accept files.

### Visual list (thumbnail + name + path)
- Image files display real thumbnails.
- Each entry shows a readable name and the file path (paths are elided to fit the UI).

### Lock (pin) items
Each item has a lock toggle:
- **🔒 Locked:** the item will **not** be auto-removed after you drag it out
- **🔓 Unlocked:** the item can be auto-removed (depending on settings)

This is helpful for “always needed” files like templates or frequently reused assets.

### Quick remove
- Each item has a small **×** button at the top-right to remove it immediately (manual remove always works, even for locked items).

---

## How to use

### Show the shelf
1. Start dragging a file (for example from File Explorer).
2. Move your cursor toward the configured screen edge.
3. The shelf will appear—drop the file to add it.

### Add via clipboard
- Copy a file in Explorer → switch to the shelf → press **Ctrl + V**
- Take a screenshot → press **Ctrl + V**
- Copy text → press **Ctrl + V** (creates a temporary `.txt`)

### Copy / drag out
- Select one or more items → **Ctrl + C** to copy them into clipboard (as file items)
- Or drag them out directly into a target folder/app

### Lock / unlock
- Click **🔒 / 🔓** on an item.
- Locked items remain on the shelf even after you drag them out.

### Remove items
- Click **×** on the item to remove it from the list instantly.
- Use the bottom “Clear (Unlocked)” button to remove all unlocked items in one go.

---

## Project structure

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


- `edge_sensor.py` handles detecting “drag-to-edge” behavior.
- `shelf_window.py` implements the shelf UI and item interactions.
- `tray.py` manages the system tray icon and related actions.
- `settings.py` stores basic preferences (dock side, behavior toggles).
- `models.py` contains the data model for shelf items.
- `utils.py` contains helper functions (thumbnails, temp files, open in explorer, etc.).

---

## Requirements

- Windows 10/11
- Python 3.10+ (3.11+ recommended)
- PySide6 (Qt for Python)
- Some Windows-specific helpers/APIs (used for input/drag detection)

---

## Run locally (Windows)

### 1) Clone the repo
```bash
git clone https://github.com/<YOUR_GITHUB_USERNAME>/MyFileStation.git
cd MyFileStation
2) Create and activate a virtual environment
PowerShell:

python -m venv .venv
.\.venv\Scripts\Activate.ps1
3) Install dependencies
pip install -r requirements.txt
4) Run the app
Because the project uses a src/ layout:

$env:PYTHONPATH="src"
python -m myfilestation.main
You should see the app start (usually with a tray icon). Then try dragging a file toward the configured edge.

Notes about drag & drop on Windows
If drag & drop does not work but clipboard import works, it is often a privilege-level issue:

If the app is running as Administrator and File Explorer is not, Windows may block drag & drop between them.

Recommended: run MyFileStation as non-admin for normal daily usage.

Roadmap (ideas)
A simple Preferences UI (dock side, remove-after-drag-out, autostart)

Better preview support (quick peek / richer preview panel)

More polished multi-select drag-out experience

Optional behavior: keep a copy vs remove after exporting

Improved file type icons for non-image files

Contributing
Issues and PRs are welcome.
If you’re filing a bug, please include:

Windows version

steps to reproduce

expected vs actual behavior

screenshots if UI-related

For PRs:

keep changes focused

add short notes in the PR description

include screenshots/GIFs for UI changes

License
MIT License. See LICENSE.