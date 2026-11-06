"""Static integration checks. These do not replace Cargo or desktop runtime tests."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

def source(name):
    return (ROOT / name).read_text()

class UniversalBackendChecks(unittest.TestCase):
    def test_backend_has_dispatched_desktops_and_uses_no_shell(self):
        code = source('crates/p3w-platform/src/linux/universal_wallpaper.rs')
        for desktop in ['Hyprland', 'Sway', 'Gnome', 'Plasma', 'Xfce', 'Cinnamon', 'Mate', 'Lxqt']:
            self.assertIn(f'Self::{desktop}', code)
        self.assertNotIn('sh -c', code)
        self.assertNotIn('bash -c', code)
        self.assertIn('Command::new(', code)

    def test_files_are_checked_before_applying(self):
        code = source('crates/p3w-platform/src/linux/universal_wallpaper.rs')
        self.assertIn('if !path.is_file()', code)
        self.assertIn('path.canonicalize()', code)
        self.assertIn('self.validate_target(monitor)?;', code)

    def test_sway_uses_output_provider_without_hyprctl(self):
        code = source('crates/p3w-platform/src/linux/sway.rs')
        self.assertIn('get_outputs', code)
        self.assertNotIn('hyprctl', code)
        old_player = source('crates/p3w-platform/src/linux/wallpaper.rs')
        self.assertIn('Desktop::Sway => SwayMonitorProvider.monitors()?', old_player)

    def test_app_controllers_use_universal_selection(self):
        for file in ['app/src/monitor_controller.rs', 'app/src/cxxqt_object.rs', 'app/src/settings_controller.rs']:
            self.assertIn('UniversalWallpaperBackend', source(file))
        self.assertIn('wallpaper_capabilities()', source('app/qml/Main.qml'))
        self.assertIn('"all"', source('crates/p3w-platform/src/linux/universal_wallpaper.rs'))

    def test_video_routes_are_capability_checked(self):
        code = source('crates/p3w-platform/src/linux/universal_wallpaper.rs')
        for route in ('ExistingWayland', 'Generic(Mode::X11)', 'Generic(Mode::WaylandLayer)', 'GnomeExtension', 'PlasmaPlugin'):
            self.assertIn(route, code)
        self.assertIn('gnome_extension_enabled()', code)
        self.assertIn('installed_plasma_plugin()', code)
        self.assertIn('Desktop settings were not changed.', code)

    def test_settings_migration_is_desktop_scoped(self):
        code = source('app/src/settings_store.rs')
        self.assertIn('wallpapers_v2', code)
        self.assertIn('backend == "hyprland"', code)
        self.assertIn('saved_wallpapers_for(backend:', code)
        self.assertIn('let mut items = Map::new()', code)

    def test_settings_disable_unsupported_controls(self):
        qml = source('app/qml/views/SettingsView.qml')
        self.assertIn('enabled: root.playbackControlsAvailable', qml)
        self.assertIn('enabled: root.focusControlsAvailable', qml)
        self.assertIn('enabled: root.soundEnabled && root.focusControlsAvailable', qml)

if __name__ == '__main__':
    unittest.main()
