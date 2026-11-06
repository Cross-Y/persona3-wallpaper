"""Static and executable smoke tests; Rust/Cargo and Qt are tested separately."""
from pathlib import Path
import os
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]

class LinuxCompatTest(unittest.TestCase):
    def source(self, rel):
        return (ROOT / rel).read_text()

    def test_path_module_is_shared(self):
        module = self.source('crates/p3w-platform/src/linux/paths.rs')
        self.assertIn('p.is_absolute()', module)
        for source in ('app/src/settings_store.rs', 'app/src/cxxqt_object.rs',
                       'app/src/url_catalog.rs', 'app/src/font_manager.rs',
                       'app/src/cursor_manager.rs', 'app/src/autostart.rs',
                       'app/src/desktop_options.rs'):
            with self.subTest(source=source):
                self.assertIn('paths::', self.source(source))

    def test_no_distro_package_manager_in_app_runtime(self):
        for f in (ROOT / 'app/src').glob('*.rs'):
            with self.subTest(file=f.name):
                code = f.read_text()
                for command in ('pacman -', 'apt-get', 'dnf install', 'zypper install'):
                    self.assertNotIn(command, code)

    def test_wallpaper_backend_is_universal_and_keeps_wlroots_engine(self):
        env = self.source('crates/p3w-platform/src/linux/environment.rs')
        self.assertIn('super::universal_wallpaper::UniversalWallpaperBackend', env)
        engine = self.source('crates/p3w-platform/src/linux/wallpaper.rs')
        self.assertIn('spawn_wallpaper(', engine)
        self.assertIn('"mpvpaper"', engine)
        self.assertIn('"swaybg"', engine)
        self.assertIn('Desktop::Sway', engine)
        self.assertIn('UniversalWallpaperBackend', self.source('app/src/monitor_controller.rs'))
        self.assertIn('backend.support(', self.source('app/src/cxxqt_object.rs'))

    def test_supported_desktop_categories_and_noctalia_overlay(self):
        env = self.source('crates/p3w-platform/src/linux/environment.rs')
        for name in ('Hyprland', 'Sway', 'Gnome', 'Plasma', 'Xfce', 'Cinnamon', 'Mate', 'Lxqt'):
            self.assertIn(name, env)
        self.assertIn('noctalia', env)

    def test_qml_warns_for_unsupported_backend(self):
        self.assertIn('platformNotice: window.platformNotice', self.source('app/qml/Main.qml'))
        self.assertIn('visible: view.platformNotice.length > 0', self.source('app/qml/views/LibraryView.qml'))

    def test_no_tray_no_hidden_window_trap(self):
        source = self.source('app/qml/Main.qml')
        self.assertIn('runtimeSettings.backgroundMode && trayIcon.available', source)
        self.assertIn('running: trayIcon.visible && trayIcon.available && !window.visible', source)

    def test_diagnostics_script_runs_without_side_effects(self):
        script = ROOT / 'scripts/check-linux-compat.sh'
        env = os.environ.copy()
        env.update(XDG_CURRENT_DESKTOP='Hyprland:Noctalia', XDG_SESSION_TYPE='wayland')
        output = subprocess.check_output(['bash', str(script)], env=env, text=True)
        self.assertIn('Hyprland:Noctalia', output)
        self.assertIn('Distribution:', output)
        self.assertIn('Wallpaper tools by desktop:', output)
        self.assertIn('pcmanfm-qt', output)
        self.assertIn('qdbus6', output)

if __name__ == '__main__':
    unittest.main()
