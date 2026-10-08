<p align="center">
  <img src="docs/assets/banner.svg" alt="MyFileStation — a place for files between apps" width="100%" />
</p>

<p align="center">
  <a href="https://github.com/JeremyL691/MyFileStation/actions/workflows/windows.yml"><img src="https://github.com/JeremyL691/MyFileStation/actions/workflows/windows.yml/badge.svg" alt="Windows quality checks" /></a>
  <a href="https://github.com/JeremyL691/MyFileStation/releases"><img src="https://img.shields.io/github/v/release/JeremyL691/MyFileStation?label=release&color=64748b" alt="Latest published release" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-64748b" alt="Windows 10 / 11" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-64748b" alt="MIT license" /></a>
</p>

<p align="center">
  <strong>Put files down for a moment. Pick them up in another app.</strong><br />
  A Windows file shelf for files, folders, and clipboard content.
</p>

<p align="center">
  <a href="README.md">简体中文</a> · English<br />
  <a href="https://github.com/JeremyL691/MyFileStation/releases">Published releases</a> ·
  <a href="#get-started">Get started</a> ·
  <a href="https://github.com/JeremyL691/MyFileStation/issues/new/choose">Report an issue</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

## A temporary home for your files

Found a file before opening the app that needs it? Drag it to the screen edge and leave it on the shelf. When you're ready, drag it out again. Paste files, text, or images to keep related content together while you work.

| Feature | What it does |
| --- | --- |
| Edge reveal | Opens at the left or right screen edge while dragging from Explorer or the desktop |
| Files and clipboard | References files and folders; saves pasted text and images as local files |
| Search and selection | Searches names and paths, filters by type, and supports multiple selection |
| Pins and persistence | Keeps important items pinned and restores entries and settings from SQLite |
| Hand off to another app | Drags in copy mode, copies file objects, or copies a path as text |
| Desktop controls | Tray menu, configurable shortcut, Chinese / English, light / dark / system theme |

**External files stay where they are.** Removing a shelf entry never deletes the original file or folder. Clipboard-generated files are managed separately and queued for delayed cleanup according to your settings.

## Downloads and release status

| Goal | Where to go |
| --- | --- |
| Use a published version | [GitHub Releases](https://github.com/JeremyL691/MyFileStation/releases); follow that version's instructions |
| Try the development version | Download `MyFileStation-windows-x64` from a successful [Windows Actions run](https://github.com/JeremyL691/MyFileStation/actions/workflows/windows.yml), or build from source |
| Review validation | [Acceptance report, Chinese](QA_REPORT.md) · [Test matrix](TESTING.md) · [Changelog](CHANGELOG.md) |

The current source is **0.3.0 at the release-candidate stage**, using Rust, Tauri 2, and React. The published **v0.2.0** uses the previous Python / PySide6 implementation and has a different interface. The full cross-app drag matrix, installation and upgrade flows, and long-running stability checks are still pending.

Windows 10 / 11 x64 and Microsoft WebView2 Runtime are required. The 0.3.0 installer uses a current-user installation and a WebView2 bootstrapper. For the portable ZIP, extract everything and run `Start-MyFileStation.cmd`; the launcher checks for WebView2. Portable application data still lives in the current user's local app-data directory.

## Get started

1. **Open the shelf** from the tray or with `Ctrl+Alt+Space`. Dragging a local file from Explorer or the desktop to the configured screen edge also reveals it.
2. **Add content** by dropping files or folders, using the add buttons, or pressing `Ctrl+V` inside the shelf.
3. **Use it elsewhere** by dragging selected entries to a supported file target or pressing `Ctrl+C` to copy file objects. Pin anything you want to keep.

By default, a successful drag removes unpinned entries from the shelf. You can change this in Settings. Dragging always uses copy mode and leaves external source files in place.

| Action | Shortcut / gesture |
| --- | --- |
| Show / hide shelf | `Ctrl+Alt+Space`, configurable |
| Paste content | `Ctrl+V` |
| Copy selected file objects | `Ctrl+C` |
| Select all visible entries | `Ctrl+A` |
| Range / additive selection | `Shift` / `Ctrl` + click |
| Open an entry | Double-click, or `Enter` on a focused row |
| Toggle selection | `Space` on a focused row |
| Remove selected entries | `Delete`; pinned content requires confirmation |
| Hide shelf | `Esc` |

## Current scope

- Automatic edge reveal supports local file drags from Explorer and the desktop. Browser images and virtual attachments are outside the current import scope.
- Drag-out support depends on the target app's file import capability. See [TESTING.md](TESTING.md) for the target-app validation matrix.
- Run as a standard user, matching Explorer's privilege level, to avoid Windows drag-and-drop restrictions.
- Builds currently target Windows x64.

## Data and privacy

The app stores SQLite data, generated clipboard files, thumbnails, and logs under `%LOCALAPPDATA%\MyFileStation\v2`. External files remain in their original locations. Entries whose source files were moved or deleted appear unavailable.

There is no telemetry, and clipboard text is not written to logs. On its first launch, the new app imports recognized settings from `%APPDATA%\MyFileStation\settings.json` while preserving that file. Legacy shelf entries are not automatically migrated.

## Development

Install Rust stable with the MSVC target, Visual Studio 2022 C++ Build Tools and Windows SDK, Node.js 22+, pnpm 11, and WebView2 Runtime on Windows 10 / 11 x64.

```powershell
git clone https://github.com/JeremyL691/MyFileStation.git
cd MyFileStation
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev` launches Vite and the native app. `pnpm --filter @myfilestation/frontend dev:web` runs only the frontend; native operations require Tauri.

```powershell
# Automated acceptance checks, including the storage stress test
./scripts/test-windows.ps1 -IncludeStress

# Current-user installer, portable ZIP, and SHA-256 checksums
./scripts/build-windows.ps1
./scripts/package-portable.ps1
```

Outputs go to `release/`. Add `-SkipInstaller` to the build command if you only need the portable version. See [Contributing, Chinese](CONTRIBUTING.md) and [Architecture, Chinese](docs/ARCHITECTURE.md) for more context.

The Python fallback remains in `src/myfilestation/`: install `requirements.txt`, then run `python run_myfilestation.py`. Its tests use `python -m pytest -q`.

## Feedback

[Report an issue or suggest a feature](https://github.com/JeremyL691/MyFileStation/issues/new/choose) in Chinese or English. Include Windows, display scaling, target-app versions, and reproducible steps. Remove private file content and sensitive paths from attachments.

[MIT License](LICENSE) · Copyright © 2026 JeremyL691
