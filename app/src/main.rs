pub mod cxxqt_object;
pub mod font_manager;
pub mod cursor_manager;
pub mod theme_manager;
pub mod theme_integrations;
pub mod monitor_controller;
pub mod settings_controller;
mod autostart;
mod desktop_options;
mod focus_monitor;
mod resource_monitor;
mod wallpaper_restore;
mod image_optimizer;
mod theme_palette;
mod theme_media;
pub mod settings_store;
pub mod url_catalog;

use cxx_qt::casting::Upcast;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQmlEngine, QUrl};
use std::pin::Pin;

fn main() {
    if std::env::args_os().any(|arg| arg.to_str() == Some("--version")) {
        println!("Persona 3 Wallpaper {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if std::env::args_os().any(|arg| arg.to_str() == Some("--help")) {
        println!("Persona 3 Wallpaper\n  --version         Print version\n  --diagnose-linux  Print desktop environment diagnostics\n  --help            Show this help");
        return;
    }
    if std::env::args_os().any(|arg| arg.to_str() == Some("--diagnose-linux")) {
        println!("{}", p3w_platform::linux::environment::diagnostics());
        return;
    }
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(
            "qrc:/qt/qml/com/persona3/wallpaper/qml/Main.qml",
        ));
    }

    if let Some(engine) = engine.as_mut() {
        let engine: Pin<&mut QQmlEngine> = engine.upcast_pin();

        engine
            .on_quit(|_| {
                // Avoid orphaning xwinwrap/mpvpaper children at process exit.
                // GNOME/Plasma plugins are compositor-managed; Hyprland retains
                // its existing pause-on-exit behavior.
                p3w_platform::linux::video_optimizer::cancel_all_preparations();
                image_optimizer::cancel_all();
                let _ = p3w_platform::linux::universal_wallpaper::UniversalWallpaperBackend::detect().on_app_exit();
                println!("Persona 3 Wallpaper: window closed");
            })
            .release();
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
