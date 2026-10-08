mod error;
mod models;
mod store;
mod tray;
mod windows;

pub use error::{AppError, AppResult};
use models::{AppSettings, ImportedItems, ShelfItem};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};
use store::ShelfStore;
use tauri::{
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    path::BaseDirectory,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{
    Builder as GlobalShortcutBuilder, GlobalShortcutExt, ShortcutState,
};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) store: Arc<Mutex<Option<ShelfStore>>>,
    pub(crate) settings: Arc<RwLock<AppSettings>>,
    pub(crate) state_version: Arc<AtomicU64>,
    pub(crate) startup_error: Option<AppError>,
    pub(crate) settings_update: Arc<tauri::async_runtime::Mutex<()>>,
}

impl AppState {
    pub(crate) fn new(settings: AppSettings, store: Option<ShelfStore>) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            settings: Arc::new(RwLock::new(settings)),
            state_version: Arc::new(AtomicU64::new(0)),
            startup_error: None,
            settings_update: Arc::new(tauri::async_runtime::Mutex::new(())),
        }
    }

    pub(crate) fn publish(&self, app: &AppHandle) {
        #[derive(Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct VersionEvent {
            version: u64,
        }

        let version = self.state_version.fetch_add(1, Ordering::AcqRel) + 1;
        let _ = app.emit("shelf-updated", VersionEvent { version });
    }
}

fn data_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(|root| PathBuf::from(root).join("AppData").join("Local"))
        })
        .unwrap_or_else(std::env::temp_dir)
        .join("MyFileStation")
        .join("v2")
}

fn create_settings_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show()?;
        window.set_focus()?;
        return Ok(window);
    }

    WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("index.html?view=settings".into()),
    )
    .title("MyFileStation Settings")
    .inner_size(760.0, 660.0)
    .min_inner_size(640.0, 520.0)
    .resizable(true)
    .visible(true)
    .skip_taskbar(false)
    .build()
}

fn set_shelf_visible(app: &AppHandle, focus: bool) -> AppResult<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::new("WINDOW_UNAVAILABLE", "error.windowUnavailable"))?;

    if !window.is_visible().unwrap_or(false) {
        let dock_side = app
            .state::<AppState>()
            .settings
            .read()
            .map(|settings| settings.dock_side)
            .unwrap_or_default();
        windows::position_shelf(&window, dock_side)?;
        window.show()?;
    }

    window.set_always_on_top(true)?;
    if focus {
        window.set_focus()?;
    }

    Ok(())
}

fn toggle_shelf_visible(app: &AppHandle, focus: bool) {
    if app
        .get_webview_window("main")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
    {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
    } else if let Err(error) = set_shelf_visible(app, focus) {
        log::error!("The shelf could not be shown: {error}");
    }
}

fn start_artifact_collector(app: AppHandle) {
    let spawn = std::thread::Builder::new()
        .name("myfilestation-artifact-collector".into())
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(6 * 60 * 60));
                let clipboard_paths = windows::read_clipboard_files().ok();
                let state = app.state::<AppState>().inner().clone();
                let Ok(mut guard) = state.store.lock() else {
                    continue;
                };
                let Some(store) = guard.as_mut() else {
                    continue;
                };
                if let Err(error) = store.collect_expired_artifacts(clipboard_paths.as_deref()) {
                    log::warn!("Periodic artifact cleanup was postponed: {error}");
                }
            }
        });
    if let Err(error) = spawn {
        log::error!("The artifact collector could not start: {error}");
    }
}

#[tauri::command]
async fn list_items(app: AppHandle) -> AppResult<Vec<ShelfItem>> {
    let state = app.state::<AppState>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let store = state
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        store
            .as_ref()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
            .list_items()
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))?
}

#[tauri::command]
async fn get_settings(app: AppHandle) -> AppResult<AppSettings> {
    let settings = app
        .state::<AppState>()
        .settings
        .read()
        .map_err(|_| AppError::new("SETTINGS_UNAVAILABLE", "error.settingsUnavailable"))?
        .clone();
    Ok(settings)
}

fn normalize_hotkey(value: &str) -> AppResult<String> {
    let mut modifiers = Vec::new();
    let mut key = None;
    for token in value
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let lower = token.to_ascii_lowercase();
        let normalized = match lower.as_str() {
            "ctrl" | "control" | "cmdorcontrol" | "commandorcontrol" => "Ctrl".to_owned(),
            "alt" | "option" => "Alt".to_owned(),
            "shift" => "Shift".to_owned(),
            "win" | "windows" | "super" | "meta" => "Super".to_owned(),
            "space" | "spacebar" => "Space".to_owned(),
            _ if token.chars().count() == 1 => token.to_ascii_uppercase(),
            _ => {
                let mut chars = token.chars();
                chars
                    .next()
                    .map(|head| head.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            }
        };
        if matches!(normalized.as_str(), "Ctrl" | "Alt" | "Shift" | "Super") {
            if !modifiers.contains(&normalized) {
                modifiers.push(normalized);
            }
        } else if key.replace(normalized).is_some() {
            return Err(AppError::new("SHORTCUT_INVALID", "error.shortcutInvalid"));
        }
    }
    let Some(key) = key else {
        return Err(AppError::new("SHORTCUT_INVALID", "error.shortcutInvalid"));
    };
    if modifiers.is_empty()
        || key.len() == 1 && !key.chars().next().is_some_and(char::is_alphanumeric)
    {
        return Err(AppError::new("SHORTCUT_INVALID", "error.shortcutInvalid"));
    }
    modifiers.sort_by_key(|modifier| match modifier.as_str() {
        "Ctrl" => 0,
        "Alt" => 1,
        "Shift" => 2,
        _ => 3,
    });
    modifiers.push(key);
    let hotkey = modifiers.join("+");
    hotkey
        .parse::<tauri_plugin_global_shortcut::Shortcut>()
        .map_err(|_| AppError::new("SHORTCUT_INVALID", "error.shortcutInvalid"))?;
    Ok(hotkey)
}

const AUTOSTART_REGISTRY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn apply_autostart(enabled: bool) -> AppResult<()> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _) = current_user
        .create_subkey(AUTOSTART_REGISTRY_PATH)
        .map_err(|error| {
            AppError::with_detail(
                "AUTOSTART_FAILED",
                "error.autostartFailed",
                error.to_string(),
            )
        })?;
    const VALUE_NAME: &str = "MyFileStation";
    if enabled {
        let executable = std::env::current_exe()?;
        let command = format!("\"{}\" --background", executable.display());
        run_key.set_value(VALUE_NAME, &command).map_err(|error| {
            AppError::with_detail(
                "AUTOSTART_FAILED",
                "error.autostartFailed",
                error.to_string(),
            )
        })?;
    } else if run_key.get_value::<String, _>(VALUE_NAME).is_ok() {
        run_key.delete_value(VALUE_NAME).map_err(|error| {
            AppError::with_detail(
                "AUTOSTART_FAILED",
                "error.autostartFailed",
                error.to_string(),
            )
        })?;
    }
    Ok(())
}

fn read_autostart_status() -> AppResult<bool> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = match current_user.open_subkey(AUTOSTART_REGISTRY_PATH) {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(AppError::with_detail(
                "AUTOSTART_STATUS_UNAVAILABLE",
                "error.autostartFailed",
                error.to_string(),
            ));
        }
    };
    let command = match run_key.get_value::<String, _>("MyFileStation") {
        Ok(command) => command,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(AppError::with_detail(
                "AUTOSTART_STATUS_UNAVAILABLE",
                "error.autostartFailed",
                error.to_string(),
            ));
        }
    };
    let current_exe = std::env::current_exe()?;
    let expected = format!("\"{}\"", current_exe.display());
    Ok(command
        .get(..expected.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(&expected)))
}

#[tauri::command]
async fn get_startup_status() -> AppResult<bool> {
    tauri::async_runtime::spawn_blocking(read_autostart_status)
        .await
        .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))?
}

#[tauri::command]
async fn update_settings(app: AppHandle, mut settings: AppSettings) -> AppResult<AppSettings> {
    settings.hotkey = normalize_hotkey(&settings.hotkey)?;
    let state = app.state::<AppState>().inner().clone();
    let _update_guard = state.settings_update.lock().await;
    let previous = state
        .settings
        .read()
        .map_err(|_| AppError::new("SETTINGS_UNAVAILABLE", "error.settingsUnavailable"))?
        .clone();
    let hotkey_changed =
        normalize_hotkey(&previous.hotkey).ok().as_deref() != Some(settings.hotkey.as_str());

    if hotkey_changed {
        if let Err(error) = app.global_shortcut().register(settings.hotkey.as_str()) {
            return Err(AppError::with_detail(
                "SHORTCUT_REGISTER_FAILED",
                "error.shortcutConflict",
                error.to_string(),
            ));
        }
        if app
            .global_shortcut()
            .is_registered(previous.hotkey.as_str())
            && let Err(error) = app.global_shortcut().unregister(previous.hotkey.as_str())
        {
            let _ = app.global_shortcut().unregister(settings.hotkey.as_str());
            return Err(AppError::with_detail(
                "SHORTCUT_UNREGISTER_FAILED",
                "error.shortcutUpdateFailed",
                error.to_string(),
            ));
        }
    }
    if settings.autostart != previous.autostart
        && let Err(error) = apply_autostart(settings.autostart)
    {
        if hotkey_changed {
            let _ = app.global_shortcut().unregister(settings.hotkey.as_str());
            let _ = app.global_shortcut().register(previous.hotkey.as_str());
        }
        return Err(error);
    }

    let persisted = settings.clone();
    let state_for_save = state.clone();
    let save_result = tauri::async_runtime::spawn_blocking(move || {
        let mut guard = state_for_save
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        guard
            .as_mut()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
            .save_settings(&persisted)
    })
    .await
    .unwrap_or_else(|_| {
        Err(AppError::new(
            "BACKGROUND_TASK_FAILED",
            "error.operationFailed",
        ))
    });
    if let Err(error) = save_result {
        if hotkey_changed {
            let _ = app.global_shortcut().unregister(settings.hotkey.as_str());
            let _ = app.global_shortcut().register(previous.hotkey.as_str());
        }
        if settings.autostart != previous.autostart {
            let _ = apply_autostart(previous.autostart);
        }
        return Err(error);
    }

    *state
        .settings
        .write()
        .map_err(|_| AppError::new("SETTINGS_UNAVAILABLE", "error.settingsUnavailable"))? =
        settings.clone();
    if let Some(window) = app.get_webview_window("main")
        && window.is_visible().unwrap_or(false)
        && let Err(error) = windows::position_shelf(&window, settings.dock_side)
    {
        log::warn!("The saved dock side could not be applied: {error}");
    }
    state.publish(&app);
    Ok(settings)
}

#[tauri::command]
async fn import_paths(app: AppHandle, paths: Vec<String>) -> AppResult<ImportedItems> {
    let state = app.state::<AppState>().inner().clone();
    let imported = tauri::async_runtime::spawn_blocking(move || {
        let mut store = state
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        let store = store
            .as_mut()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        let imported = store.import_paths(paths)?;
        Ok::<_, AppError>(imported)
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;

    if !imported.items.is_empty() {
        app.state::<AppState>().publish(&app);
        set_shelf_visible(&app, false)?;
    }

    Ok(imported)
}

#[tauri::command]
async fn paste_clipboard(app: AppHandle) -> AppResult<ImportedItems> {
    let paths = tauri::async_runtime::spawn_blocking(windows::read_clipboard_files)
        .await
        .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;
    if !paths.is_empty() {
        return import_paths(app, paths).await;
    }

    let image_app = app.clone();
    let image_result = tauri::async_runtime::spawn_blocking(move || {
        let image = image_app
            .clipboard()
            .read_image()
            .map_err(|error| error.to_string())?;
        Ok::<_, String>((image.width(), image.height(), image.rgba().to_vec()))
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))?;
    if let Ok((width, height, rgba)) = image_result {
        let item = create_image_item(app, width, height, rgba).await?;
        return Ok(ImportedItems {
            items: vec![item],
            duplicate_count: 0,
            invalid_count: 0,
        });
    }

    let text_app = app.clone();
    let text_result = tauri::async_runtime::spawn_blocking(move || {
        text_app
            .clipboard()
            .read_text()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))?;
    match text_result {
        Ok(text) if !text.trim().is_empty() => {
            let item = create_text_item(app, text).await?;
            Ok(ImportedItems {
                items: vec![item],
                duplicate_count: 0,
                invalid_count: 0,
            })
        }
        _ => Ok(ImportedItems {
            items: Vec::new(),
            duplicate_count: 0,
            invalid_count: 0,
        }),
    }
}

#[tauri::command]
async fn create_text_item(app: AppHandle, text: String) -> AppResult<ShelfItem> {
    let state = app.state::<AppState>().inner().clone();
    let item = tauri::async_runtime::spawn_blocking(move || {
        let mut store = state
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        store
            .as_mut()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
            .create_text_item(&text)
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;
    app.state::<AppState>().publish(&app);
    set_shelf_visible(&app, false)?;
    Ok(item)
}

#[tauri::command]
async fn create_image_item(
    app: AppHandle,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> AppResult<ShelfItem> {
    let state = app.state::<AppState>().inner().clone();
    let item = tauri::async_runtime::spawn_blocking(move || {
        let mut store = state
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        store
            .as_mut()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
            .create_image_item(width, height, &rgba)
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;
    app.state::<AppState>().publish(&app);
    set_shelf_visible(&app, false)?;
    Ok(item)
}

#[tauri::command]
async fn set_pinned(app: AppHandle, item_id: String, pinned: bool) -> AppResult<ShelfItem> {
    let state = app.state::<AppState>().inner().clone();
    let item = tauri::async_runtime::spawn_blocking(move || {
        let mut store = state
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        store
            .as_mut()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
            .set_pinned(&item_id, pinned)
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;
    app.state::<AppState>().publish(&app);
    Ok(item)
}

#[tauri::command]
async fn remove_items(
    app: AppHandle,
    item_ids: Vec<String>,
    force: bool,
) -> AppResult<Vec<ShelfItem>> {
    let state = app.state::<AppState>().inner().clone();
    let removed = tauri::async_runtime::spawn_blocking(move || {
        let mut store = state
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        store
            .as_mut()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
            .remove_items(&item_ids, force)
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;

    if !removed.is_empty() {
        app.state::<AppState>().publish(&app);
    }
    Ok(removed)
}

#[tauri::command]
async fn handle_drag_out_result(
    app: AppHandle,
    item_ids: Vec<String>,
    succeeded: bool,
) -> AppResult<Vec<ShelfItem>> {
    if !succeeded {
        return Ok(Vec::new());
    }

    let auto_remove = app
        .state::<AppState>()
        .settings
        .read()
        .map_err(|_| AppError::new("SETTINGS_UNAVAILABLE", "error.settingsUnavailable"))?
        .remove_after_drag_out;
    if !auto_remove {
        return Ok(Vec::new());
    }

    remove_items(app, item_ids, false).await
}

#[tauri::command]
async fn clear_unlocked(app: AppHandle) -> AppResult<Vec<ShelfItem>> {
    let ids = {
        let state = app.state::<AppState>().inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            let store = state
                .store
                .lock()
                .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
            store
                .as_ref()
                .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
                .unpinned_ids()
        })
        .await
        .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??
    };
    remove_items(app, ids, false).await
}

#[tauri::command]
async fn get_clipboard_files() -> AppResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(windows::read_clipboard_files)
        .await
        .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))?
}

#[tauri::command]
async fn copy_paths_to_clipboard(
    app: AppHandle,
    window: WebviewWindow,
    item_ids: Vec<String>,
) -> AppResult<usize> {
    let state = app.state::<AppState>().inner().clone();
    let paths = tauri::async_runtime::spawn_blocking(move || {
        let store = state
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        store
            .as_ref()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
            .existing_paths(&item_ids)
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;

    let path_count = paths.len();
    let owner = window.hwnd()?.0 as isize;
    tauri::async_runtime::spawn_blocking(move || windows::write_clipboard_files(&paths, owner))
        .await
        .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;
    Ok(path_count)
}

#[tauri::command]
async fn copy_path_to_clipboard(app: AppHandle, item_id: String) -> AppResult<()> {
    let state = app.state::<AppState>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = {
            let guard = state
                .store
                .lock()
                .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
            guard
                .as_ref()
                .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
                .existing_path(&item_id)?
        };
        app.clipboard()
            .write_text(path.to_string_lossy().into_owned())
            .map_err(|error| {
                AppError::with_detail(
                    "CLIPBOARD_WRITE_FAILED",
                    "error.clipboardBusy",
                    error.to_string(),
                )
            })
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))?
}

#[tauri::command]
fn drag_icon_path(app: AppHandle) -> AppResult<String> {
    let bundled = app
        .path()
        .resolve("resources/drag-icon.png", BaseDirectory::Resource);
    if let Ok(path) = bundled
        && path.is_file()
    {
        return Ok(path.to_string_lossy().into_owned());
    }
    let local = std::env::current_dir()
        .unwrap_or_default()
        .join("icons")
        .join("drag-icon.png");
    if local.is_file() {
        return Ok(local.to_string_lossy().into_owned());
    }
    Err(AppError::new(
        "DRAG_ICON_UNAVAILABLE",
        "error.dragCouldNotStart",
    ))
}

#[tauri::command]
async fn open_item(app: AppHandle, item_id: String) -> AppResult<()> {
    let state = app.state::<AppState>().inner().clone();
    let path = tauri::async_runtime::spawn_blocking(move || {
        let store = state
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        store
            .as_ref()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
            .existing_path(&item_id)
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;
    windows::open_path(&path)
}

#[tauri::command]
async fn reveal_item(app: AppHandle, item_id: String) -> AppResult<()> {
    let state = app.state::<AppState>().inner().clone();
    let path = tauri::async_runtime::spawn_blocking(move || {
        let store = state
            .store
            .lock()
            .map_err(|_| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?;
        store
            .as_ref()
            .ok_or_else(|| AppError::new("STORAGE_UNAVAILABLE", "error.storageUnavailable"))?
            .existing_path(&item_id)
    })
    .await
    .map_err(|_| AppError::new("BACKGROUND_TASK_FAILED", "error.operationFailed"))??;
    windows::reveal_path(&path)
}

#[tauri::command]
fn show_shelf(app: AppHandle) -> AppResult<()> {
    set_shelf_visible(&app, true)
}

#[tauri::command]
fn hide_shelf(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide()?;
    }
    Ok(())
}

#[tauri::command]
fn toggle_shelf(app: AppHandle) {
    toggle_shelf_visible(&app, true);
}

#[tauri::command]
async fn show_settings(app: AppHandle) -> AppResult<()> {
    create_settings_window(&app)
        .map(|_| ())
        .map_err(AppError::from)
}

#[tauri::command]
fn window_ready(app: AppHandle, window: WebviewWindow) -> AppResult<()> {
    if window.label() == "main" {
        windows::start_edge_watcher(app.clone());
        if !std::env::args().any(|argument| argument == "--background")
            && let Err(error) = set_shelf_visible(&app, false)
        {
            log::warn!("The shelf could not be shown on launch: {error}");
        }
    }
    if let Some(error) = &app.state::<AppState>().startup_error {
        return Err(error.clone());
    }
    Ok(())
}

#[tauri::command]
async fn quit_app(app: AppHandle) -> AppResult<()> {
    app.exit(0);
    Ok(())
}

pub fn run() {
    let plugins = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Err(error) = set_shelf_visible(app, true) {
                log::warn!("The existing shelf could not be shown: {error}");
            }
        }))
        .plugin(
            GlobalShortcutBuilder::new()
                .with_handler(|app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_shelf_visible(app, true);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Folder {
                        path: data_root().join("logs"),
                        file_name: Some("MyFileStation".into()),
                    },
                )])
                .build(),
        );

    let app = plugins
        .invoke_handler(tauri::generate_handler![
            list_items,
            get_settings,
            get_startup_status,
            update_settings,
            import_paths,
            paste_clipboard,
            create_text_item,
            create_image_item,
            set_pinned,
            remove_items,
            handle_drag_out_result,
            clear_unlocked,
            get_clipboard_files,
            copy_paths_to_clipboard,
            copy_path_to_clipboard,
            drag_icon_path,
            open_item,
            reveal_item,
            show_shelf,
            hide_shelf,
            toggle_shelf,
            show_settings,
            window_ready,
            quit_app
        ])
        .setup(|app| {
            let root = data_root();
            let artifacts = root.join("artifacts");
            let thumbnails = root.join("thumbnails");
            let logs = root.join("logs");
            std::fs::create_dir_all(&artifacts)?;
            std::fs::create_dir_all(&thumbnails)?;
            std::fs::create_dir_all(&logs)?;

            let (store, settings, startup_error) =
                match ShelfStore::open(root.clone(), artifacts, thumbnails) {
                    Ok((store, settings)) => (Some(store), settings, None),
                    Err(error) => {
                        log::error!("The shelf database could not be opened: {error}");
                        (None, AppSettings::default(), Some(error))
                    }
                };

            let mut state = AppState::new(settings, store);
            state.startup_error = startup_error;
            app.manage(state);
            let settings = app
                .state::<AppState>()
                .settings
                .read()
                .map(|settings| settings.clone())
                .unwrap_or_default();
            if let Err(error) = app.global_shortcut().register(settings.hotkey.as_str()) {
                log::warn!("The global shortcut could not be registered: {error}");
            }
            if app.state::<AppState>().startup_error.is_none()
                && let Err(error) = apply_autostart(settings.autostart)
            {
                log::warn!("The startup setting could not be applied: {error}");
            }
            if let Ok(mut guard) = app.state::<AppState>().store.lock()
                && let Some(store) = guard.as_mut()
            {
                if let Err(error) = store.reconcile_orphaned_artifacts() {
                    log::warn!("Managed temporary files could not be reconciled: {error}");
                }
                let clipboard_paths = windows::read_clipboard_files().ok();
                if let Err(error) = store.collect_expired_artifacts(clipboard_paths.as_deref()) {
                    log::warn!("Expired temporary files could not be collected: {error}");
                }
            }
            tray::install(app.handle())?;
            windows::start_edge_watcher(app.handle().clone());
            start_artifact_collector(app.handle().clone());

            if !std::env::args().any(|arg| arg == "--background") {
                set_shelf_visible(app.handle(), true)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!());

    match app {
        Ok(app) => app.run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let state = app_handle.state::<AppState>().inner().clone();
                let cleanup = state
                    .settings
                    .read()
                    .map(|settings| settings.cleanup_temp_on_exit)
                    .unwrap_or(true);
                if cleanup
                    && let Ok(mut guard) = state.store.lock()
                    && let Some(store) = guard.as_mut()
                    && let Err(error) = store.cleanup_unpinned()
                {
                    log::error!("Exit cleanup could not finish: {error}");
                }
            }
        }),
        Err(error) => {
            eprintln!("MyFileStation failed to start: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_use_a_consistent_modifier_order_and_accept_legacy_aliases() {
        assert_eq!(
            normalize_hotkey(" option + control + SPACEBAR ").unwrap(),
            "Ctrl+Alt+Space"
        );
        assert_eq!(
            normalize_hotkey("Ctrl+Ctrl+Alt+space").unwrap(),
            "Ctrl+Alt+Space"
        );
    }

    #[test]
    fn invalid_hotkeys_are_rejected_before_registration() {
        for value in [
            "",
            "Ctrl+Alt",
            "Space",
            "Ctrl+A+B",
            "Ctrl+NotAKey",
            "Ctrl+!",
        ] {
            assert_eq!(
                normalize_hotkey(value).unwrap_err().code,
                "SHORTCUT_INVALID"
            );
        }
    }
}
