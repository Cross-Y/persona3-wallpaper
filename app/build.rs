use cxx_qt_build::{CxxQtBuilder, QmlModule};
use std::{env, fs, path::{Path, PathBuf}};

// Embed whichever supported image is present; leave the QML resource name
// extensionless so Qt detects the image type from its contents.
// This also produces a useful error instead of the opaque "Failed to write .qrc".
fn artwork(stem: &str) -> PathBuf {
    const EXTENSIONS: [&str; 4] = ["webp", "png", "jpeg", "jpg"];
    let available: Vec<PathBuf> = EXTENSIONS.iter()
        .map(|ext| PathBuf::from(format!("assets/{stem}.{ext}")))
        .filter(|file| file.is_file())
        .collect();

    if available.is_empty() {
        panic!("Missing image for '{stem}' in app/assets/. Add {stem}.png, {stem}.jpg, {stem}.jpeg or {stem}.webp.");
    }
    if available.len() > 1 {
        println!("cargo:warning=Multiple images found for {stem}; using {}. Remove unused versions to avoid ambiguity.", available[0].display());
    }
    available[0].clone()
}

fn xml_escape(input: &str) -> String {
    input.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&apos;")
}

fn add_resource(xml: &mut String, alias: &str, file: &Path) {
    let path = fs::canonicalize(file).unwrap_or_else(|error|
        panic!("Cannot access image {}: {error}", file.display()));
    xml.push_str(&format!("    <file alias=\"{}\">{}</file>\n",
        xml_escape(alias), xml_escape(&path.to_string_lossy())));
}

fn main() {
    println!("cargo:rerun-if-changed=assets");
    let mut xml = String::from("<RCC>\n  <qresource prefix=\"/qt/qml/com/persona3/wallpaper\">\n");

    for stem in ["persona3-background", "persona3-url", "persona3-appearance", "persona3-settings"] {
        add_resource(&mut xml, &format!("assets/{stem}"), &artwork(stem));
    }
    // The tray icon still uses its original, explicitly named PNG resource.
    add_resource(&mut xml, "assets/persona3-evoker.png", Path::new("assets/persona3-evoker.png"));
    // Optional QML sound bank + short, original PCM WAV clips.
    // Loading the QtMultimedia component is deferred until runtime.
    add_resource(&mut xml, "audio/MenuSoundEffects.qml", Path::new("qml/components/MenuSoundEffects.qml"));
    for clip in ["ui-hover.wav", "ui-select.wav", "ui-open.wav", "ui-close.wav", "ui-tab.wav"] {
        add_resource(&mut xml, &format!("audio/{clip}"), &PathBuf::from(format!("assets/ui/{clip}")));
    }
    println!("cargo:rerun-if-changed=qml/components/MenuSoundEffects.qml");
    xml.push_str("  </qresource>\n</RCC>\n");

    // cxx-qt-build uses the QRC filename as part of a C++ Q_INIT_RESOURCE
    // identifier. Hyphens are invalid there; use underscores instead.
    let qrc_file = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"))
        .join("persona3_artwork.qrc");
    fs::write(&qrc_file, xml).expect("Cannot write generated artwork QRC");

    CxxQtBuilder::new_qml_module(
        QmlModule::new("com.persona3.wallpaper")
            .qml_file("qml/Main.qml")
            .qml_file("qml/components/AppBackground.qml")
            .qml_file("qml/components/ClockTransition.qml")
            .qml_file("qml/components/Sidebar.qml")
            .qml_file("qml/components/ImportDialog.qml")
            .qml_file("qml/components/WallpaperHeader.qml")
            .qml_file("qml/components/WallpaperCard.qml")
            .qml_file("qml/components/EmptyLibrary.qml")
            .qml_file("qml/components/VideoPreparationDialog.qml")
            .qml_file("qml/components/ImageFramingDialog.qml")
            .qml_file("qml/views/LibraryView.qml")
            .qml_file("qml/views/SettingsView.qml")
            .qml_file("qml/views/AppearanceView.qml")
            .qml_file("qml/views/UrlView.qml"),
    )
    .qrc(&qrc_file)
    .files([
        "src/cxxqt_object.rs",
        "src/monitor_controller.rs",
        "src/settings_controller.rs",
    ])
    .build();
}
