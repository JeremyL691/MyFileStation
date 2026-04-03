# Testing MyFileStation

## Automated Checks

Install dependencies:

```powershell
python -m pip install -r requirements.txt
```

Run the full automated suite:

```powershell
python -m pytest -q
```

Run only the Qt smoke tests:

```powershell
python -m pytest -m smoke
```

The Qt tests default to `QT_QPA_PLATFORM=offscreen`, so they can run without an interactive desktop session.

## Windows Manual Regression Checklist

Environment baseline:

- Windows 10 or Windows 11
- Run once on a single-display setup
- Run once on a dual-display setup
- Run as a normal user
- Validate both Explorer and Desktop drag sources

Checklist:

- [ ] Launch with `python run_myfilestation.py` and confirm the app stays running
- [ ] Verify the tray icon appears and left-click toggles shelf visibility
- [ ] Switch dock side left/right and confirm the shelf repositions immediately
- [ ] Toggle `Auto-remove After Drag Out`, restart, and confirm the setting persists
- [ ] Toggle `Clean Up Temp Items On Exit`, restart, and confirm the setting persists
- [ ] Toggle `Auto-start with Windows` and confirm it can be enabled and disabled without errors
- [ ] Drag a file from Explorer to the configured edge and confirm the shelf reveals
- [ ] Drag a file from the Desktop to the configured edge and confirm the shelf reveals
- [ ] Cancel an edge drag before drop and confirm an empty shelf auto-hides
- [ ] Drop a local file into the shelf and confirm it renders with the correct icon or thumbnail
- [ ] Press `Ctrl+V` with copied files and confirm they are imported
- [ ] Press `Ctrl+V` with copied text and confirm a temporary text item is created
- [ ] Press `Ctrl+V` with a screenshot and confirm a temporary image item is created
- [ ] Double-click an item and confirm it opens
- [ ] Press `Enter` and `Space` on a selected item and confirm it opens
- [ ] Press `Delete` on selected mixed items and confirm pinned items are preserved
- [ ] Pin an item, drag it out successfully, and confirm it is not auto-removed
- [ ] Run `Clear Unlocked` and confirm pinned items remain
- [ ] Use the right-click menu to open, reveal in Explorer, copy path, pin/unpin, remove, and force remove
- [ ] Delete a backing file externally, then try open/reveal/copy/drag-out and confirm the app warns and removes the stale shelf entry
- [ ] Exit the app with unpinned temporary items present and confirm they are deleted when cleanup-on-exit is enabled
- [ ] Start the app as Administrator and confirm the drag-and-drop warning appears

## Release Gate

Run before every release:

1. `python -m pytest -q`
2. `python -m pytest -m smoke`
3. Complete the Windows manual regression checklist
4. Validate the packaged build separately before publishing
