use std::sync::atomic::Ordering;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle,
};

use crate::palette;

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
    let quit = MenuItem::with_id(app, "quit", "Quit Conjure", true, Some("Cmd+Q"))?;
    let menu = Menu::with_items(app, &[&switch, &show_minimized, &quit])?;

    let tray_icon =
        tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

    TrayIconBuilder::new()
        .icon(tray_icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app: &AppHandle, event| match event.id.as_ref() {
            "switch" => palette::toggle(app),
            "show_minimized" => {
                let current = palette::INCLUDE_MINIMIZED.load(Ordering::Relaxed);
                palette::INCLUDE_MINIMIZED.store(!current, Ordering::Relaxed);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}
