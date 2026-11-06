"""Ensure the optional Qt menu audio doesn't regress UI or resource handling."""
import pathlib
import wave
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class MenuSoundTests(unittest.TestCase):
    def test_replaceable_pcm_clips_decode(self):
        for name in ("ui-hover.wav", "ui-select.wav"):
            p = ROOT / "app/assets/ui" / name
            self.assertGreater(p.stat().st_size, 44)
            with wave.open(str(p), "rb") as stream:
                self.assertIn(stream.getnchannels(), (1, 2))
                self.assertEqual(stream.getsampwidth(), 2)
                self.assertLess(stream.getnframes() / stream.getframerate(), 2.0)

    def test_audio_qrc_resources_and_safe_cpp_identifier(self):
        build = (ROOT / "app/build.rs").read_text()
        self.assertIn('"audio/MenuSoundEffects.qml"', build)
        self.assertIn('"ui-hover.wav", "ui-select.wav"', build)
        self.assertIn('persona3_artwork.qrc', build)
        self.assertNotIn('persona3-artwork.qrc', build)

    def test_player_is_optional_and_short_lived(self):
        main = (ROOT / "app/qml/Main.qml").read_text()
        player = (ROOT / "app/qml/components/MenuSoundEffects.qml").read_text()
        self.assertIn('active: window.visible && runtimeSettings.uiSoundsEnabled', main)
        self.assertIn('item.soundVolume = runtimeSettings.uiSoundVolume', main)
        self.assertNotIn('import QtMultimedia', main)
        self.assertIn('import QtMultimedia', player)
        self.assertEqual(player.count('SoundEffect {'), 5)
        self.assertIn('now - lastUiHoverMs < 85', main)
        self.assertIn('qrc:/qt/qml/com/persona3/wallpaper/audio/ui-hover.wav', player)

    def test_persistent_controls_separate_from_video_audio(self):
        store = (ROOT / "app/src/settings_store.rs").read_text()
        controller = (ROOT / "app/src/settings_controller.rs").read_text()
        settings = (ROOT / "app/qml/views/SettingsView.qml").read_text()
        self.assertIn('"ui_sounds_enabled":true', store)
        self.assertIn('"ui_sound_volume":0.55', store)
        self.assertIn('fn set_ui_sounds(', controller)
        self.assertIn('fn set_ui_sound_volume(', controller)
        self.assertIn('Menu sound effects', settings)
        self.assertIn('Menu sound volume', settings)

    def test_coverage_across_the_main_sections(self):
        for file in ('Main.qml', 'components/Sidebar.qml', 'views/SettingsView.qml',
                     'views/AppearanceView.qml', 'views/UrlView.qml',
                     'views/LibraryView.qml', 'components/WallpaperCard.qml',
                     'components/WallpaperHeader.qml'):
            source = (ROOT / 'app/qml' / file).read_text()
            self.assertIn('uihover', source.lower(), file)
            self.assertIn('uiselect', source.lower(), file)


if __name__ == '__main__':
    unittest.main()
