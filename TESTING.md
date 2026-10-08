# Testing MyFileStation

## Automated checks

For a fail-fast local run, use `./scripts/test-windows.ps1 -IncludeStress`. The optional stress check creates 1,000 external references, measures 50 full list reads, exercises 1,000 generated text/image lifecycles, reopens SQLite, and validates deferred cleanup. It is a storage test; it does not substitute for native drag cycles or the 8-hour release gate.

The latest local findings and remaining release gates are recorded in [QA_REPORT.md](./QA_REPORT.md).

Run these from the repository root on Windows:

```powershell
pnpm install --frozen-lockfile
pnpm test
pnpm typecheck
pnpm lint
pnpm format:check
pnpm --filter @myfilestation/frontend build:web
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The Vitest suite covers filtering, selection, keyboard removal, and settings rollback. Rust tests cover SQLite creation and migration, persistence and recovery, external path protection, pinned-item rules, clipboard-generated files, and deferred cleanup. The legacy Python suite is a migration reference and can still be run with `python -m pytest -q`.

## Windows compatibility matrix

Record the Windows edition/build, display topology and scaling, WebView2 version, and each target application's version with every manual run. Run as a standard user. Test Windows 10 and 11; one and two displays; 100%, 125%, 150%, and 200% scaling; mixed DPI; displays left of the primary display; and monitor removal/reconnection.

### Shelf and settings

- [ ] Shelf opens in the work area of the cursor's display and stays within its bounds.
- [ ] Left/right docking, tray show/hide, close-to-hide, settings window, and global shortcut work.
- [ ] Shortcut conflicts are reported and the previous shortcut stays registered.
- [ ] Chinese and English labels, light/dark/system themes, keyboard focus, and long names render correctly.
- [ ] Search matches names and paths; changing a filter clears selection.
- [ ] Multi-select, `Ctrl+A`, `Ctrl+C`, `Ctrl+V`, `Enter`, `Space`, `Delete`, and `Esc` work.
- [ ] Context menu supports open, reveal, copy path, pin/unpin, and remove.
- [ ] Settings persist after restart; failed writes restore the previous UI value.
- [ ] Start-with-Windows status matches the current user's Run registry entry.

### Import, paste, and copy drag-out

- [ ] Import a single file, folder, multi-selection, Chinese name, space, long path, and repeated path from Explorer and the desktop.
- [ ] Folders remain references; their contents are not scanned or copied.
- [ ] Paste copied files, text, and images. Verify generated images open and can be pasted or dragged into supported targets.
- [ ] Missing and temporarily unavailable paths remain visible as unavailable entries.
- [ ] Drag copies to Explorer, Chrome, Edge, WeChat, QQ, VS Code, and supported Office file targets.
- [ ] Cancel a drag, reject a drop, and test a target that reads the file asynchronously. Originals remain unchanged.
- [ ] Confirm a successful drag only removes the shelf entry when the setting is enabled, and pinned items remain.
- [ ] Exit immediately after a drag and verify that temporary data remains available to any active clipboard reference.

### Recovery and lifecycle

- [ ] Restart with pinned and ordinary active entries; verify valid entries restore.
- [ ] Enable exit cleanup and verify only unpinned generated items are queued for cleanup.
- [ ] Confirm that remove, clear, and cleanup never delete an external file or recurse into an external folder.
- [ ] Interrupt generated-file writing, database commit, and cleanup; restart and verify recovery or an actionable error.
- [ ] Simulate a corrupt database and confirm the original is retained and startup reports the problem.
- [ ] Launch a second instance and confirm it reveals the existing shelf.
- [ ] Restart Explorer and verify edge reveal recovers.
- [ ] Toggle auto-start and confirm there is only one effective startup entry.

### Target application scope

Use each app's normal, supported file import flow. For WeChat and QQ, use their own file-transfer or “My Computer” destinations. Do not treat unsupported virtual attachments or browser image downloads as failures in this release.

## Release gates

- [ ] Automated checks pass in local development and Windows CI.
- [ ] Complete and record the compatibility matrix above on the release candidate.
- [ ] Run for 8 hours and complete 1,000 drag-in, drag-out, and cancellation cycles without sustained resource growth.
- [ ] On the recorded SSD test machine, measure shelf reveal P95 at or below 200 ms and search P95 at or below 100 ms for 1,000 items.
- [ ] Install and uninstall the NSIS package in a clean environment; validate upgrade and rollback.
- [ ] Run the portable ZIP from a clean environment and validate missing-WebView2 guidance.
- [ ] Record package SHA-256 values and attach the compatibility report to the release.

The developer machine can run automated and interactive checks, but the multi-application matrix, 8-hour run, and 1,000-cycle stress gate require a release-candidate QA session and should not be marked complete until recorded.
