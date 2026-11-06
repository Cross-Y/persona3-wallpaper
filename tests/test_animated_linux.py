"""Static contract checks; a real desktop and `cargo check` are still required."""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
def read(path): return (ROOT / path).read_text()

class AnimatedLinuxIntegration(unittest.TestCase):
    def test_separate_generic_player_and_compositor_plugins(self):
        u = read('crates/p3w-platform/src/linux/universal_wallpaper.rs')
        for name in ('VideoRoute::ExistingWayland', 'VideoRoute::Generic', 'VideoRoute::GnomeExtension', 'VideoRoute::PlasmaPlugin'):
            self.assertIn(name, u)
        self.assertIn('wallpaperengine@waylandwe', u)
        self.assertIn('luisbocanegra.smart.video.wallpaper.reborn', u)
    def test_x11_spawn_and_cleanup(self):
        code = read('crates/p3w-platform/src/linux/animated_video.rs')
        self.assertIn('Command::new("xwinwrap")', code)
        self.assertIn('"p3w-xwinwrap", "WID"', code)
        self.assertIn('command.process_group(0)', code)
        self.assertIn('libc::kill(-pgid, libc::SIGTERM)', code)
        self.assertIn('mpv_ipc(&player.socket, "pause", paused)', code)
    def test_static_switch_stops_generic_and_disables_gnome_video(self):
        code = read('crates/p3w-platform/src/linux/universal_wallpaper.rs')
        self.assertIn('GenericVideoBackend::stop_all()?;', code)
        self.assertIn('gnome_setting("enabled", "false")?;', code)
    def test_gif_is_video_and_has_preview(self):
        code = read('app/src/cxxqt_object.rs')
        self.assertIn('"avi" | "gif"', code)
        self.assertIn('ensure_thumbnail(&destination', code)
    def test_settings_and_exit_dispatch(self):
        settings = read('app/src/settings_controller.rs')
        self.assertIn('universal.pause_all()', settings)
        self.assertIn('universal.resume_all()', settings)
        self.assertIn('UniversalWallpaperBackend::detect().stop_all()', settings)
        self.assertIn('on_app_exit()', read('app/src/main.rs'))
    def test_diagnostic_has_no_side_effects(self):
        code = read('scripts/check-animated-linux.sh')
        self.assertNotIn('sudo ', code)
        self.assertNotIn('apt install', code)
        self.assertNotIn('pacman -S', code)
        self.assertIn('command -v', code)

if __name__ == '__main__': unittest.main()
