use crate::models::DockSide;
use crate::{AppError, AppResult};
use std::{
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
use windows::Win32::{
    Foundation::{GlobalFree, HANDLE, POINT, RECT},
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
        },
        Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
    },
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON},
        Shell::{DragQueryFileW, HDROP},
        WindowsAndMessaging::{
            GA_ROOT, GetAncestor, GetClassNameW, GetCursorPos, GetParent, GetSystemMetrics,
            SM_CXDRAG, SM_CYDRAG, SW_SHOWNORMAL, WindowFromPoint,
        },
    },
};

static EDGE_WATCHER_RUNNING: AtomicBool = AtomicBool::new(false);
const CF_HDROP_FORMAT: u32 = 15;
const EDGE_THRESHOLD_DIP: i32 = 48;
const DRAG_HELD_MS: u128 = 60;
const REVEAL_COOLDOWN: Duration = Duration::from_millis(350);

pub fn position_shelf(window: &WebviewWindow, side: DockSide) -> AppResult<()> {
    let monitors = window.available_monitors()?;
    let cursor = cursor_position();
    let monitor = cursor
        .and_then(|(x, y)| {
            monitors.iter().find(|monitor| {
                let position = monitor.position();
                let size = monitor.size();
                x >= position.x
                    && y >= position.y
                    && x < position.x.saturating_add(size.width as i32)
                    && y < position.y.saturating_add(size.height as i32)
            })
        })
        .or_else(|| monitors.first());

    let Some(monitor) = monitor else {
        return Ok(());
    };
    let area = monitor.work_area();
    let work_area = RECT {
        left: area.position.x,
        top: area.position.y,
        right: area.position.x.saturating_add(area.size.width as i32),
        bottom: area.position.y.saturating_add(area.size.height as i32),
    };
    let (position, size) = shelf_bounds(work_area, monitor.scale_factor(), side);
    // Moving first lets Windows apply the destination monitor's DPI before sizing.
    window.set_position(position)?;
    window.set_min_size(Some(PhysicalSize::new(
        size.width.min((360.0 * monitor.scale_factor()) as u32),
        size.height.min((440.0 * monitor.scale_factor()) as u32),
    )))?;
    window.set_size(size)?;
    window.set_position(position)?;
    Ok(())
}

fn shelf_bounds(
    work_area: RECT,
    scale_factor: f64,
    side: DockSide,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let scale_factor = scale_factor.max(0.5);
    let width_px = (388.0 * scale_factor).round() as u32;
    let height_px = (660.0 * scale_factor).round() as u32;
    let margin_px = (8.0 * scale_factor).round() as i32;
    let width_px = width_px.min((work_area.right - work_area.left).max(1) as u32);
    let height_px = height_px.min((work_area.bottom - work_area.top).max(1) as u32);
    let x = match side {
        DockSide::Left => work_area.left + margin_px,
        DockSide::Right => work_area.right - width_px as i32 - margin_px,
    }
    .clamp(
        work_area.left,
        (work_area.right - width_px as i32).max(work_area.left),
    );
    let y = (work_area.top + margin_px).clamp(
        work_area.top,
        (work_area.bottom - height_px as i32).max(work_area.top),
    );
    (
        PhysicalPosition::new(x, y),
        PhysicalSize::new(width_px, height_px),
    )
}

fn cursor_position() -> Option<(i32, i32)> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).ok()? };
    Some((point.x, point.y))
}

pub fn read_clipboard_files() -> AppResult<Vec<String>> {
    unsafe {
        OpenClipboard(None).map_err(|error| {
            AppError::with_detail("CLIPBOARD_BUSY", "error.clipboardBusy", error.to_string())
        })?;
        let _guard = ClipboardGuard;
        let data = GetClipboardData(CF_HDROP_FORMAT);
        let Ok(data) = data else {
            return Ok(Vec::new());
        };
        if data.is_invalid() {
            return Ok(Vec::new());
        }

        let drop = HDROP(data.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        let mut paths = Vec::with_capacity(count as usize);
        for index in 0..count {
            let mut buffer = vec![0u16; 32_768];
            let length = DragQueryFileW(drop, index, Some(&mut buffer));
            if length == 0 || length as usize >= buffer.len() {
                continue;
            }
            buffer.truncate(length as usize);
            paths.push(String::from_utf16_lossy(&buffer));
        }
        Ok(paths)
    }
}

pub fn write_clipboard_files(paths: &[PathBuf], owner: isize) -> AppResult<()> {
    if owner == 0 {
        return Err(AppError::new(
            "WINDOW_UNAVAILABLE",
            "error.windowUnavailable",
        ));
    }
    if paths.is_empty() {
        return Err(AppError::new(
            "CLIPBOARD_FILES_EMPTY",
            "error.noAvailableItems",
        ));
    }

    let mut filenames = Vec::new();
    for path in paths {
        let path = dunce::canonicalize(path)?;
        if !path.is_absolute() {
            return Err(AppError::new("PATH_INVALID", "error.pathInvalid"));
        }
        let wide = path.as_os_str().encode_wide();
        if wide.clone().any(|unit| unit == 0) {
            return Err(AppError::new("PATH_INVALID", "error.pathInvalid"));
        }
        filenames.extend(wide);
        filenames.push(0);
    }
    filenames.push(0);

    #[repr(C)]
    struct DropFiles {
        file_list_offset: u32,
        x: i32,
        y: i32,
        non_client: i32,
        wide: i32,
    }

    let header = std::mem::size_of::<DropFiles>();
    let bytes = header + filenames.len() * std::mem::size_of::<u16>();
    let block = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }.map_err(|error| {
        AppError::with_detail(
            "CLIPBOARD_ALLOCATION_FAILED",
            "error.clipboardBusy",
            error.to_string(),
        )
    })?;
    let pointer = unsafe { GlobalLock(block) } as *mut u8;
    if pointer.is_null() {
        let _ = unsafe { GlobalFree(Some(block)) };
        return Err(AppError::new(
            "CLIPBOARD_ALLOCATION_FAILED",
            "error.clipboardBusy",
        ));
    }

    unsafe {
        let drop_files = DropFiles {
            file_list_offset: header as u32,
            x: 0,
            y: 0,
            non_client: 0,
            wide: 1,
        };
        std::ptr::write_unaligned(pointer.cast::<DropFiles>(), drop_files);
        std::ptr::copy_nonoverlapping(
            filenames.as_ptr().cast::<u8>(),
            pointer.add(header),
            filenames.len() * std::mem::size_of::<u16>(),
        );
        let _ = GlobalUnlock(block);
        if let Err(error) = OpenClipboard(Some(windows::Win32::Foundation::HWND(
            owner as *mut std::ffi::c_void,
        ))) {
            let _ = GlobalFree(Some(block));
            return Err(AppError::with_detail(
                "CLIPBOARD_BUSY",
                "error.clipboardBusy",
                error.to_string(),
            ));
        }
        let _guard = ClipboardGuard;
        if let Err(error) = EmptyClipboard() {
            let _ = GlobalFree(Some(block));
            return Err(AppError::with_detail(
                "CLIPBOARD_BUSY",
                "error.clipboardBusy",
                error.to_string(),
            ));
        }
        if let Err(error) = SetClipboardData(CF_HDROP_FORMAT, Some(HANDLE(block.0))) {
            let _ = GlobalFree(Some(block));
            return Err(AppError::with_detail(
                "CLIPBOARD_WRITE_FAILED",
                "error.clipboardBusy",
                error.to_string(),
            ));
        }
    }
    Ok(())
}

struct ClipboardGuard;

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

pub fn open_path(path: &Path) -> AppResult<()> {
    if !Path::try_exists(path)? {
        return Err(AppError::new("FILE_MISSING", "error.fileUnavailable"));
    }
    use windows::Win32::{Foundation::HWND, UI::Shell::ShellExecuteW};
    use windows::core::PCWSTR;
    let verb: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let target: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            PCWSTR(verb.as_ptr()),
            PCWSTR(target.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as usize <= 32 {
        Err(AppError::with_detail(
            "OPEN_FAILED",
            "error.openFailed",
            format!("Windows ShellExecute failed ({})", result.0 as isize),
        ))
    } else {
        Ok(())
    }
}

pub fn reveal_path(path: &Path) -> AppResult<()> {
    if !Path::try_exists(path)? {
        return Err(AppError::new("FILE_MISSING", "error.fileUnavailable"));
    }
    std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            AppError::with_detail("REVEAL_FAILED", "error.revealFailed", error.to_string())
        })
}

#[derive(Default)]
struct DragSource {
    down_at: Option<Instant>,
    down_point: POINT,
    dragging_from_shell_file_view: bool,
    has_revealed: bool,
    last_reveal: Option<Instant>,
}

fn toggle_left_button_down() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 }
}

fn begin_drag_from_shell_file_view(point: POINT) -> bool {
    unsafe {
        let window = WindowFromPoint(point);
        if window.is_invalid() {
            return false;
        }
        let mut current = window;
        let mut found_file_view = false;
        for _ in 0..10 {
            if current.is_invalid() {
                break;
            }
            let mut class_name = [0u16; 128];
            let written = GetClassNameW(current, &mut class_name);
            let class = String::from_utf16_lossy(&class_name[..written.max(0) as usize]);
            if ["DirectUIHWND", "SysListView32", "SHELLDLL_DefView"]
                .iter()
                .any(|class_name| class.eq_ignore_ascii_case(class_name))
            {
                found_file_view = true;
            }
            let Ok(parent) = GetParent(current) else {
                break;
            };
            current = parent;
        }
        if !found_file_view {
            return false;
        }
        let root = GetAncestor(window, GA_ROOT);
        let mut root_class = [0u16; 128];
        let root_written = GetClassNameW(root, &mut root_class);
        let root_class = String::from_utf16_lossy(&root_class[..root_written.max(0) as usize]);
        ["CabinetWClass", "ExploreWClass", "Progman", "WorkerW"]
            .iter()
            .any(|supported| root_class.eq_ignore_ascii_case(supported))
    }
}

fn near_edge(point: POINT, side: DockSide) -> bool {
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_invalid() {
        return false;
    }
    let info = unsafe {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        info
    };
    let dpi = unsafe {
        let window = WindowFromPoint(point);
        if window.is_invalid() {
            96
        } else {
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(window);
            if dpi == 0 { 96 } else { dpi }
        }
    };
    let threshold = (EDGE_THRESHOLD_DIP as u32 * dpi / 96) as i32;
    if info.rcWork.right <= info.rcWork.left {
        return false;
    }
    match side {
        DockSide::Left => point.x <= info.rcWork.left + threshold,
        DockSide::Right => point.x >= info.rcWork.right - threshold,
    }
}

fn update_edge_drag(app: &AppHandle, source: &mut DragSource) {
    let mut point = POINT::default();
    unsafe {
        if GetCursorPos(&mut point).is_err() {
            return;
        }
    }

    let down = toggle_left_button_down();
    let Some(started) = source.down_at else {
        if down {
            source.down_at = Some(Instant::now());
            source.down_point = point;
            source.dragging_from_shell_file_view = begin_drag_from_shell_file_view(point);
            source.has_revealed = false;
        }
        return;
    };

    if !down {
        source.down_at = None;
        source.dragging_from_shell_file_view = false;
        source.has_revealed = false;
        return;
    }

    if !source.dragging_from_shell_file_view || source.has_revealed {
        return;
    }
    let drag_distance =
        unsafe { GetSystemMetrics(SM_CXDRAG) + GetSystemMetrics(SM_CYDRAG) }.max(12);
    let moved = (point.x - source.down_point.x).abs() + (point.y - source.down_point.y).abs()
        >= drag_distance;
    if !moved || started.elapsed().as_millis() < DRAG_HELD_MS {
        return;
    }

    let settings = app
        .state::<crate::AppState>()
        .settings
        .read()
        .map(|settings| settings.clone())
        .unwrap_or_default();
    if !near_edge(point, settings.dock_side)
        || source
            .last_reveal
            .is_some_and(|last| last.elapsed() < REVEAL_COOLDOWN)
    {
        return;
    }

    source.last_reveal = Some(Instant::now());
    source.has_revealed = true;
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = position_shelf(&window, settings.dock_side) {
            log::warn!("The shelf could not be positioned at the drag edge: {error}");
        }
        if let Err(error) = window.show() {
            log::warn!("The shelf could not be revealed during a file drag: {error}");
        }
        let _ = window.set_always_on_top(true);
    }
}

pub fn start_edge_watcher(app: AppHandle) {
    if EDGE_WATCHER_RUNNING.swap(true, Ordering::AcqRel) {
        return;
    }
    if let Err(error) = thread::Builder::new()
        .name("myfilestation-edge-sensor".into())
        .spawn(move || {
            let mut source = DragSource::default();
            while EDGE_WATCHER_RUNNING.load(Ordering::Acquire) {
                update_edge_drag(&app, &mut source);
                thread::sleep(Duration::from_millis(16));
            }
        })
    {
        EDGE_WATCHER_RUNNING.store(false, Ordering::Release);
        log::error!("The edge watcher could not start: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelf_bounds_stay_within_negative_and_small_work_areas_at_every_scale() {
        for area in [
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040,
            },
            RECT {
                left: -1920,
                top: -1080,
                right: 0,
                bottom: 0,
            },
            RECT {
                left: 100,
                top: 40,
                right: 420,
                bottom: 340,
            },
        ] {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                for side in [DockSide::Left, DockSide::Right] {
                    let (position, size) = shelf_bounds(area, scale, side);
                    assert!(position.x >= area.left && position.y >= area.top);
                    assert!(position.x + size.width as i32 <= area.right);
                    assert!(position.y + size.height as i32 <= area.bottom);
                }
            }
        }
    }

    #[test]
    fn docking_uses_the_requested_monitor_scale() {
        let area = RECT {
            left: -1920,
            top: 40,
            right: 0,
            bottom: 1080,
        };
        let (right, size) = shelf_bounds(area, 1.25, DockSide::Right);
        assert_eq!(size.width, 485);
        assert_eq!(right.x, -495);
        let (left, _) = shelf_bounds(area, 1.25, DockSide::Left);
        assert_eq!(left.x, -1910);
    }
}
