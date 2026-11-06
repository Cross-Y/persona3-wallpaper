"""Source safety contracts; compile and runtime memory testing remain necessary."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "app/src"

def read(name): return (SRC / name).read_text()

class StabilityRefactorTests(unittest.TestCase):
    def test_qml_bridge_has_delegated_desktop_services(self):
        controller = read("settings_controller.rs")
        main = read("main.rs")
        for name in ("autostart", "desktop_options", "focus_monitor", "resource_monitor", "wallpaper_restore"):
            self.assertIn("mod " + name + ";", main)
            self.assertIn("crate::" + name + "::", controller)
        self.assertLess(len(controller.splitlines()), 340)

    def test_palette_calculation_is_separated_from_ffmpeg_and_desktop(self):
        main = read("main.rs")
        theme = read("theme_manager.rs")
        palette = read("theme_palette.rs")
        media = read("theme_media.rs")
        self.assertIn("mod theme_palette;", main)
        self.assertIn("mod theme_media;", main)
        self.assertIn("palette_from_rgb", palette)
        self.assertNotIn("Command::new", palette)
        self.assertIn("sample_rgb", media)
        self.assertIn("theme_palette::", theme)

    def test_config_writes_are_atomic_and_serialized(self):
        store = read("settings_store.rs")
        for marker in ("create_new(true)", "file.sync_all()", "fs::rename(&temp, &path)",
                       "SETTINGS_UPDATE.lock()", "fn update(change:", "save_unlocked(&value)"):
            self.assertIn(marker, store)

    def test_catalog_reuses_connections_and_cleans_partial_downloads(self):
        network = read("url_catalog.rs")
        for marker in ("static CLIENT: OnceLock", "connect_timeout", "create_new(true)",
                       "MAX_DOWNLOAD_BYTES + 1", "std::fs::remove_file(&dest)"):
            self.assertIn(marker, network)

    def test_bounded_focus_buffer_and_cached_fonts(self):
        self.assertIn("pending.len() > 64 * 1024", read("focus_monitor.rs"))
        self.assertIn("FONT_CACHE", read("desktop_options.rs"))
        self.assertIn("Duration::from_secs(30)", read("desktop_options.rs"))

    def test_palette_samples_are_cached_by_source_fingerprint(self):
        theme = read("theme_manager.rs")
        for marker in ("fn source_fingerprint(path:", "theme_palette_fingerprint",
                       "theme_palette_source", "Palette::from_json(&cfg"):
            self.assertIn(marker, theme)

    def test_video_thumbnails_refresh_on_source_modification(self):
        thumbnail = (ROOT / "crates/p3w-core/src/thumbnail.rs").read_text()
        self.assertIn("thumbnail_time >= video_time", thumbnail)
        self.assertIn("fs::rename(&temporary_path, thumbnail_path)", thumbnail)
        self.assertIn("NEXT_THUMBNAIL", thumbnail)

    def test_wallpaper_backends_skip_unchanged_running_process(self):
        platform = ROOT / "crates/p3w-platform/src/linux"
        for name in ("wallpaper.rs", "animated_video.rs"):
            source = (platform / name).read_text()
            self.assertIn("file_identity(path)", source)
            self.assertIn("if unchanged", source)
            self.assertIn("active.child.try_wait()", source)
        self.assertIn("mod file_identity;", (platform / "mod.rs").read_text())

    def test_animated_frame_cache_is_bounded(self):
        media = read("theme_media.rs")
        self.assertIn("MAX_FRAMES: usize = 128", media)
        self.assertIn("prune_frame_cache(&cache, &frame)", media)

    def test_all_ui_audio_assets_are_preserved(self):
        for stem in ("hover", "select", "tab", "open", "close"):
            self.assertTrue((ROOT / "app/assets/ui" / ("ui-" + stem + ".wav")).is_file())

if __name__ == "__main__": unittest.main()
