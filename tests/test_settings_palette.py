"""Focused regression checks for synchronized settings and editable wallpaper palettes."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
SETTINGS=(ROOT/'app/qml/views/SettingsView.qml').read_text()
APPEARANCE=(ROOT/'app/qml/views/AppearanceView.qml').read_text()
THEME=(ROOT/'app/src/theme_manager.rs').read_text()
CTRL=(ROOT/'app/src/settings_controller.rs').read_text()
STORE=(ROOT/'app/src/settings_store.rs').read_text()
ADAPTERS=(ROOT/'app/src/theme_integrations.rs').read_text()

class SettingsPaletteRegression(unittest.TestCase):
    def test_toggle_state_is_authoritative(self):
        self.assertIn('component AccentSwitch: Button', SETTINGS)
        self.assertIn('checkable: false', SETTINGS)
        self.assertIn('active: root.engineActive', SETTINGS)
        self.assertIn('active: root.soundEnabled', SETTINGS)
        self.assertIn('root.appState[propertyName] = value', SETTINGS)
        self.assertNotIn('control.checked =', SETTINGS)
        self.assertNotIn('onToggled:', SETTINGS)
    def test_audio_rolls_back_failed_configuration(self):
        self.assertIn('settings_store::set_audio(old_enabled,old_desktop_only)', CTRL)
        self.assertIn('set_autostart(previous)', CTRL)
    def test_appearance_controls_are_backend_driven(self):
        self.assertIn('component ThemeChoice: Button', APPEARANCE)
        self.assertIn('checkable: false', APPEARANCE)
        self.assertIn('root.dynamicEnabled = Boolean(info.enabled)', APPEARANCE)
        self.assertIn('root.paletteOverrides = info.overrides', APPEARANCE)
        self.assertNotIn('onToggled:', APPEARANCE)
    def test_four_role_controls_and_rgb_picker(self):
        for role in ('primary','secondary','tertiary','quaternary'):
            self.assertIn(f'key:"{role}"', APPEARANCE)
            self.assertIn(f'"{role}"', THEME)
        for operation in ('root.togglePaletteManual(role)','root.openColorEditor(role)','root.setPaletteColor(colorEditor.role'):
            self.assertIn(operation,APPEARANCE)
        self.assertIn('Slider {',APPEARANCE)
    def test_palette_overrides_are_explicit_and_persisted(self):
        self.assertIn('"theme_manual_colors":{}', STORE)
        self.assertIn('pub fn set_palette_color(role: &str, color: &str)', THEME)
        self.assertIn('settings_store::save(&cfg)?',THEME)
        self.assertIn('"theme_last_palette"',THEME)
        self.assertIn('set_palette_color(self: Pin<&mut SettingsController>',CTRL)
        self.assertIn('fn valid_hex(value: &str)',(ROOT/'app/src/theme_palette.rs').read_text())
    def test_exports_include_four_roles(self):
        for role in ('secondary','tertiary','quaternary'):
            self.assertIn(f'p3w_{role}',THEME)
            self.assertIn(f'p3w_{role}',ADAPTERS)
        self.assertIn('exported["source"]',THEME)
    def test_noctalia_file_owned_and_reversible(self):
        for token in ('"version":2','"generated_by":NOCTALIA_OWNER','remove_noctalia_managed()',
                      '"custom",NOCTALIA_MANAGED','"outline_variant":p.quaternary'):
            self.assertIn(token,THEME)
        self.assertIn('previous != format!("custom {NOCTALIA_MANAGED}")',THEME)

if __name__=='__main__': unittest.main()
