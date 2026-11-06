"""Regression guards for asynchronous monitor-aware media optimization."""
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PLATFORM = ROOT / "crates/p3w-platform/src/linux"
SRC = (PLATFORM / "video_optimizer.rs").read_text()
BACKEND = (PLATFORM / "universal_wallpaper.rs").read_text()
CONTROLLER = (ROOT / "app/src/settings_controller.rs").read_text()
MAIN = (ROOT / "app/qml/Main.qml").read_text()
SETTINGS = (ROOT / "app/qml/views/SettingsView.qml").read_text()

class VideoOptimizationTests(unittest.TestCase):
    def test_monitor_detection_is_safe(self):
        self.assertIn("video_display_size", BACKEND)
        self.assertIn("largest_x11_display", SRC)
        self.assertIn("None\n    }", BACKEND)
        self.assertIn("force_original_aspect_ratio=decrease", SRC)
        self.assertIn("force_divisible_by=2", SRC)

    def test_async_cancellation_and_no_duplicate_jobs(self):
        for token in ("sync_channel::<Job>(4)", "s.queued.insert(cache.clone())", "!wanted(&job.cache)",
                      "child.kill()", "prune_cache(cache)", "APPLY_LOCK", "activate_optimized"):
            self.assertTrue(token in SRC or token in BACKEND, token)

    def test_immutable_source_and_atomic_result(self):
        self.assertIn("fs::rename(&temp, &job.cache)", SRC)
        self.assertIn(".tmp.mp4", SRC)
        self.assertNotIn("fs::rename(&temp, &job.source)", SRC)
        self.assertIn("-c:v", SRC)
        self.assertIn("libx264", SRC)
        self.assertIn("-preset", SRC)

    def test_settings_are_round_tripped(self):
        store = (ROOT / "app/src/settings_store.rs").read_text()
        for text in ('"auto_video_optimize":true', 'set_auto_video_optimize', 'video_optimization_status'):
            self.assertTrue(text in store or text in CONTROLLER, text)
        self.assertIn("autoVideoOptimize", MAIN)
        self.assertIn("autoVideoOptimize", SETTINGS)
        self.assertIn('saveToggle("set_auto_video_optimize"', SETTINGS)

    def test_no_downscaling_when_display_unknown(self):
        self.assertIn("size.filter(|v| v.valid())", SRC)
        self.assertIn("return source.to_path_buf()", SRC)
        self.assertIn("VideoRoute::ExistingWayland | VideoRoute::Generic(_)", BACKEND)

if __name__ == "__main__": unittest.main()
