use cxx_qt_lib::QString;
use p3w_platform::linux::universal_wallpaper::UniversalWallpaperBackend;
use serde_json::json;
use std::pin::Pin;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = super::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type MonitorController = super::MonitorControllerRust;

        #[qinvokable]
        fn list_monitors(self: Pin<&mut MonitorController>) -> QString;
    }
}

#[derive(Default)]
pub struct MonitorControllerRust;

impl qobject::MonitorController {
    pub fn list_monitors(self: Pin<&mut Self>) -> QString {
        let provider = UniversalWallpaperBackend::detect();
        let monitors = match provider.list_monitors() {
            Ok(monitors) => monitors,
            Err(error) => {
                eprintln!("Monitor detection failed: {error}");
                return QString::from("[]");
            }
        };

        let data: Vec<_> = monitors
            .iter()
            .map(|monitor| {
                json!({
                    "id": monitor.id,
                    "name": monitor.name,
                    "description": monitor.description,
                    "width": monitor.width,
                    "height": monitor.height,
                    "refresh_rate": monitor.refresh_rate,
                    "x": monitor.x,
                    "y": monitor.y,
                    "focused": monitor.focused
                })
            })
            .collect();

        match serde_json::to_string(&data) {
            Ok(json) => QString::from(json),
            Err(error) => {
                eprintln!("Monitor serialization failed: {error}");
                QString::from("[]")
            }
        }
    }
}
