use std::sync::atomic::Ordering;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Manager,
};

use crate::palette;

#[cfg(target_os = "macos")]
fn launch_at_login_enabled() -> bool {
    use objc2_service_management::{SMAppService, SMAppServiceStatus};
    let status = unsafe { SMAppService::mainAppService().status() };
    status == SMAppServiceStatus::Enabled || status == SMAppServiceStatus::RequiresApproval
}

#[cfg(target_os = "macos")]
fn set_launch_at_login(enable: bool) {
    use objc2_service_management::SMAppService;
    let service = unsafe { SMAppService::mainAppService() };
    let result = if enable {
        unsafe { service.registerAndReturnError() }
    } else {
        unsafe { service.unregisterAndReturnError() }
    };
    if let Err(e) = result {
        eprintln!("launch-at-login toggle failed: {e:?}");
    }
}

pub fn setup(app: &App) -> tauri::Result<()> {
    let switch = MenuItem::with_id(app, "switch", "Switch Windows", true, None::<&str>)?;
    let show_minimized = CheckMenuItem::with_id(
        app,
        "show_minimized",
        "Show Minimized Windows",
        true,
        palette::INCLUDE_MINIMIZED.load(Ordering::Relaxed),
        None::<&str>,
    )?;

    #[cfg(target_os = "macos")]
    let launch_at_login = CheckMenuItem::with_id(
        app,
        "launch_at_login",
        "Launch at Login",
        true,
        launch_at_login_enabled(),
        None::<&str>,
    )?;

    let quit = MenuItem::with_id(app, "quit", "Quit Conjure", true, Some("Cmd+Q"))?;

    #[cfg(target_os = "macos")]
    let menu = Menu::with_items(app, &[&switch, &show_minimized, &launch_at_login, &quit])?;
    #[cfg(not(target_os = "macos"))]
    let menu = Menu::with_items(app, &[&switch, &show_minimized, &quit])?;

    let tray_icon =
        tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

    #[cfg(target_os = "macos")]
    app.manage(launch_at_login);

    TrayIconBuilder::with_id("main_tray")
        .icon(tray_icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app: &AppHandle, event| match event.id.as_ref() {
            "switch" => palette::toggle(app),
            "show_minimized" => {
                let current = palette::INCLUDE_MINIMIZED.load(Ordering::Relaxed);
                palette::INCLUDE_MINIMIZED.store(!current, Ordering::Relaxed);
            }
            "launch_at_login" => {
                #[cfg(target_os = "macos")]
                {
                    let new_state = !launch_at_login_enabled();
                    set_launch_at_login(new_state);
                    if let Some(item) = app.try_state::<CheckMenuItem<tauri::Wry>>() {
                        let _ = item.set_checked(new_state);
                    }
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}
