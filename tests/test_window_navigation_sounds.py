"""Regression checks for app/window/tab cues and Qt resource packaging."""
import pathlib
import re
import unittest
import wave

ROOT = pathlib.Path(__file__).resolve().parents[1]

class WindowNavigationSoundTests(unittest.TestCase):
    def test_user_replaceable_audio_clips_exist(self):
        for name in ("ui-open.wav", "ui-close.wav", "ui-tab.wav"):
            path = ROOT / "app/assets/ui" / name
            self.assertTrue(path.is_file(), name)
            with wave.open(str(path), "rb") as clip:
                self.assertIn(clip.getnchannels(), (1, 2))
                self.assertEqual(clip.getsampwidth(), 2)
                self.assertGreater(clip.getnframes(), 0)
                self.assertLess(clip.getnframes()/clip.getframerate(), 1.8)

    def test_qrc_embeds_all_five_clips(self):
        text = (ROOT / "app/build.rs").read_text()
        self.assertIn('"audio/{clip}"', text)
        for clip in ("ui-hover.wav", "ui-select.wav", "ui-open.wav", "ui-close.wav", "ui-tab.wav"):
            self.assertIn('"' + clip + '"', text)
        self.assertIn("persona3_artwork.qrc", text)
        self.assertNotIn("persona3-artwork.qrc", text)

    def test_open_sound_is_configurable_and_not_before_settings_load(self):
        text = (ROOT / "app/qml/Main.qml").read_text()
        self.assertIn("if (!window.visible || !window.settingsInitialized || !runtimeSettings.uiSoundsEnabled)", text)
        self.assertIn("requestOpenSound()", text)
        self.assertIn("window.flushOpenSound()", text)
        self.assertIn("onVisibleChanged:", text)
        self.assertIn("active: window.visible && runtimeSettings.uiSoundsEnabled", text)

    def test_tabs_use_separate_sound_without_double_select(self):
        main = (ROOT / "app/qml/Main.qml").read_text()
        sidebar = (ROOT / "app/qml/components/Sidebar.qml").read_text()
        self.assertRegex(main, r'function navigateTo\(page\)[\s\S]*?playTabSound\(\)\s+clockTransition\.play\(\)')
        self.assertIn("if (modelData.label === root.currentPage) root.uiSelect()", sidebar)
        self.assertIn("else root.pageSelected(modelData.label)", sidebar)

    def test_close_sound_plays_before_hide_or_quit(self):
        main = (ROOT / "app/qml/Main.qml").read_text()
        player = (ROOT / "app/qml/components/MenuSoundEffects.qml").read_text()
        self.assertIn("function beginCloseWithSound(forceQuit)", main)
        self.assertIn("window.finishCloseWindow()", main)
        self.assertIn("onCloseFinished()", main)
        self.assertIn("closeSoundFallback", main)
        self.assertIn("close.accepted = false", main)
        self.assertIn("onTriggered: window.quitApp()", main)
        self.assertIn("onPlayingChanged:", player)
        self.assertIn("soundBank.closeFinished()", player)

    def test_optional_module_and_existing_controls_preserved(self):
        main = (ROOT / "app/qml/Main.qml").read_text()
        player = (ROOT / "app/qml/components/MenuSoundEffects.qml").read_text()
        settings = (ROOT / "app/qml/views/SettingsView.qml").read_text()
        self.assertNotIn("import QtMultimedia", main)
        self.assertIn("import QtMultimedia", player)
        self.assertIn("Menu sound effects", settings)
        self.assertIn("Menu sound volume", settings)
        for fx in ("playHover", "playSelect", "playOpen", "playClose", "playTab"):
            self.assertIn("function " + fx + "(", player)

if __name__ == "__main__": unittest.main()
