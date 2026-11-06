"""Static integration checks. Cargo unit tests exercise Rust palette logic."""
from pathlib import Path
import subprocess, unittest
ROOT=Path(__file__).resolve().parents[1]
APP=(ROOT/'app/src/theme_integrations.rs').read_text()
THEME=(ROOT/'app/src/theme_manager.rs').read_text()
QML=(ROOT/'app/qml/views/AppearanceView.qml').read_text()
STORE=(ROOT/'app/src/settings_store.rs').read_text()

class ThemeAdapters(unittest.TestCase):
    def test_new_module_registered(self):
        self.assertIn('pub mod theme_integrations;', (ROOT/'app/src/main.rs').read_text())
        self.assertIn('theme_integrations::apply(target,&palette)',THEME)
        self.assertIn('theme_integrations::undo(target)',THEME)
    def test_target_controls_have_defaults_and_ui(self):
        for key in ('qt','waybar','rofi','wofi'):
            self.assertIn(f'"theme_{key}_enabled":false',STORE)
            self.assertIn(f'toggleTarget("{key}", !root.',QML)
            self.assertIn(f'"{key}"=>"theme_{key}_enabled"',THEME)
    def test_outputs_have_expected_formats(self):
        for name in ('persona3-wallpaper.conf','Persona3Wallpaper.colors','persona3-wallpaper-colors.css','persona3-wallpaper-colors.rasi'):
            self.assertIn(name,APP)
        for k in ('active_colors=','inactive_colors=','disabled_colors=','[ColorScheme]','[Colors:Window]'):
            self.assertIn(k,APP)
    def test_preserves_existing_files_and_no_shell(self):
        for token in ('START:', 'END:', 'write_owned(', 'remove_owned(', 'safe_file(', 'import_block('):
            self.assertIn(token,APP)
        self.assertNotIn('Command::new("sh")',APP)
        self.assertNotIn('Command::new("bash")',APP)
        self.assertIn('cleanup(true,noctalia_enabled(&cfg),desktop_enabled(&cfg),MANAGED_TARGETS)',THEME)
    def test_writable_adapters_are_explicitly_opt_in(self):
        self.assertIn('"theme_auto_enabled":false',STORE)
        self.assertIn('if cfg[key].as_bool().unwrap_or(false)',THEME)
    def test_diagnostic_is_read_only(self):
        script=ROOT/'scripts/check-theme-integrations.sh'
        r=subprocess.run(['bash',str(script)],check=True,text=True,capture_output=True)
        self.assertIn('Color Integration Check',r.stdout)
        self.assertNotIn('sudo ',script.read_text())

if __name__=='__main__': unittest.main()
