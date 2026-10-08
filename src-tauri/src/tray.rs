use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show MyFileStation", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &settings, &separator, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("icons/icon.png".into()))?;

    TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .icon(icon)
        .tooltip("MyFileStation")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let dock_side = app
                        .state::<crate::AppState>()
                        .settings
                        .read()
                        .map(|settings| settings.dock_side)
                        .unwrap_or_default();
                    let _ = crate::windows::position_shelf(&window, dock_side);
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "settings" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = crate::show_settings(app).await {
                        log::error!("Settings could not be shown: {error}");
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
                && let Some(app) = tray.app_handle().get_webview_window("main")
            {
                if app.is_visible().unwrap_or(false) {
                    let _ = app.hide();
                } else {
                    let dock_side = tray
                        .app_handle()
                        .state::<crate::AppState>()
                        .settings
                        .read()
                        .map(|settings| settings.dock_side)
                        .unwrap_or_default();
                    let _ = crate::windows::position_shelf(&app, dock_side);
                    let _ = app.show();
                    let _ = app.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
