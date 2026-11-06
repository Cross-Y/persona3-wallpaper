"""Regression checks for the desktop recoloring master switch."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
APPEARANCE = (ROOT / "app/qml/views/AppearanceView.qml").read_text()
THEME = (ROOT / "app/src/theme_manager.rs").read_text()
STORE = (ROOT / "app/src/settings_store.rs").read_text()

class RecolorMasterTests(unittest.TestCase):
    def test_explicit_on_off_master_switch(self):
        self.assertIn('labelText: "Desktop recoloring"', APPEARANCE)
        self.assertIn('showPowerState: true', APPEARANCE)
        self.assertIn('choice.active ? "ON" : "OFF"', APPEARANCE)
        self.assertIn('onClicked: root.toggleDynamicColors(!root.dynamicEnabled)', APPEARANCE)
        self.assertIn('root.dynamicEnabled = Boolean(info.enabled)', APPEARANCE)
        self.assertIn('checkable: false', APPEARANCE)

    def test_master_defaults_off_and_persists_without_erasing_preferences(self):
        self.assertIn('"theme_auto_enabled":false', STORE)
        self.assertRegex(THEME, r'pub fn set_auto\(enabled: bool\)')
        self.assertIn('settings_store::set_bool("theme_auto_enabled",false)?;', THEME)
        self.assertIn('settings_store::set_bool("theme_auto_enabled",true)?;', THEME)
        self.assertIn('if !auto_enabled(&cfg) { return Ok("Dynamic colors disabled".into()); }', THEME)
        disable = THEME.split('pub fn set_auto(enabled: bool)', 1)[1].split('pub fn set_target(',1)[0]
        self.assertNotIn('set_bool("theme_gtk_enabled"', disable)
        self.assertNotIn('set_bool("theme_noctalia_enabled"', disable)
        self.assertNotIn('theme_manual_colors', disable)

    def test_off_restores_managed_integration_styles(self):
        self.assertIn('const MANAGED_TARGETS: &[&str] = &["qt","waybar","rofi","wofi"];', THEME)
        self.assertIn('cleanup(true,noctalia_enabled(&cfg),desktop_enabled(&cfg),MANAGED_TARGETS)', THEME)
        self.assertIn('fn undo_gtk()', THEME)
        self.assertIn('fn undo_noctalia()', THEME)
        self.assertIn('fn undo_gnome()', THEME)
        self.assertIn('fn undo_plasma()', THEME)
        self.assertIn('if managed || (current=="wallpaper m3-content" && !previous.is_empty())', THEME)

    def test_targets_remain_configurable_while_master_off(self):
        self.assertIn('Selections below are saved but inactive', APPEARANCE)
        self.assertIn('Target saved; enable dynamic colors to use it', THEME)
        self.assertIn('Color saved; it will be applied when dynamic colors are enabled', THEME)

if __name__ == "__main__":
    unittest.main()
