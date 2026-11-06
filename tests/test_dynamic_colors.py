"""Source-level safeguards; Rust unit tests also exist and should run with Cargo."""
from pathlib import Path
import unittest
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[1]
THEME = (ROOT/'app/src/theme_manager.rs').read_text()

class DynamicColorsIntegration(unittest.TestCase):
    def test_wired_to_app_and_ui(self):
        main = (ROOT/'app/src/main.rs').read_text()
        controllers = (ROOT/'app/src/cxxqt_object.rs').read_text() + (ROOT/'app/src/settings_controller.rs').read_text()
        ui = (ROOT/'app/qml/views/AppearanceView.qml').read_text()
        self.assertIn('pub mod theme_manager;', main)
        for phrase in ('theme_manager::apply_for_wallpaper','set_dynamic_colors','set_dynamic_theme_target','refresh_dynamic_colors'):
            self.assertIn(phrase,controllers)
        for phrase in ('toggleDynamicColors','toggleTarget','regenerateColors','Dynamic wallpaper colors'):
            self.assertIn(phrase.lower(),ui.lower())
    def test_opt_in_and_undo_paths(self):
        s=(ROOT/'app/src/settings_store.rs').read_text()
        self.assertIn('"theme_auto_enabled":false',s)
        for phrase in ('fn undo_gtk()', 'fn undo_gnome()', 'fn undo_plasma()', 'fn undo_noctalia()', 'if !auto_enabled(&cfg)'):
            self.assertIn(phrase,THEME)
        self.assertIn('safe_file(&css)?',THEME)
        self.assertIn('noctalia_state_settings()',THEME)
        self.assertIn('NOCTALIA_CONFIG_HOME',THEME)
    def test_command_invocation_is_not_shell(self):
        self.assertIn('Command::new(binary).args(args)',THEME)
        self.assertNotIn('sh -c',THEME)
        self.assertNotIn('Command::new("sh")',THEME)
        self.assertNotIn('Command::new("bash")',THEME)
    def test_wallpaper_backend_is_unchanged(self):
        s=(ROOT/'app/src/cxxqt_object.rs').read_text()
        self.assertIn('UniversalWallpaperBackend::detect()',s)
        self.assertIn('let was_paused = backend.playback_controls()',s)
    def test_diagnostic_is_read_only(self):
        script=ROOT/'scripts/check-dynamic-colors.sh'
        result=subprocess.run(['bash',str(script)],capture_output=True,text=True,check=True)
        self.assertIn('ffmpeg',result.stdout)
        self.assertIn('Noctalia',result.stdout)
        self.assertNotIn('sudo ',script.read_text())
    def test_ffmpeg_can_decode_64px_sample(self):
        try: import PIL.Image
        except ImportError: self.skipTest('Pillow unavailable')
        if subprocess.run(['bash','-lc','command -v ffmpeg'],capture_output=True).returncode:
            self.skipTest('ffmpeg unavailable')
        with tempfile.TemporaryDirectory() as tmp:
            image=Path(tmp)/'sample.png'
            PIL.Image.new('RGB',(32,24),(40,70,180)).save(image)
            result=subprocess.run(['ffmpeg','-nostdin','-hide_banner','-loglevel','error','-i',str(image),'-frames:v','1','-vf','scale=64:64:flags=area,format=rgb24','-pix_fmt','rgb24','-f','rawvideo','-'],capture_output=True,check=True)
            self.assertEqual(len(result.stdout),64*64*3)

if __name__=='__main__': unittest.main()
