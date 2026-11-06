"""Regression checks: visible copy and diagnostics should remain in English."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
QML = ROOT / 'app' / 'qml'
NON_ENGLISH_WORDS = re.compile(
    r'\b(?:Primario|Secundario|Terciario|Cuaternario|Automático|Apariencia|'
    r'Biblioteca|Fijar|Editar|Guardar|No\s+se\s+pudo|No\s+se\s+puede|'
    r'Colores\s+dinámicos|Integraciones\s+del\s+escritorio|'
    r'Actualizar\s+colores|Descarga\s+cancelada|Sin\s+adaptador\s+compatible)\b',
    flags=re.IGNORECASE
)

class EnglishCopyTests(unittest.TestCase):
    def test_qml_string_literals_are_english(self):
        for path in QML.rglob('*.qml'):
            with self.subTest(file=path.name):
                source = '\n'.join(line for line in path.read_text().splitlines()
                                   if not line.lstrip().startswith('//'))
                strings = re.findall(r'"(?:\\.|[^"\\])*"', source)
                joined = ' '.join(strings)
                self.assertIsNone(NON_ENGLISH_WORDS.search(joined))
                self.assertNotRegex(joined, '[áéíóúñÁÉÍÓÚÑ¿¡]')

    def test_palette_labels_and_tooltips_are_english(self):
        source = (QML / 'views' / 'AppearanceView.qml').read_text()
        for label in ('Primary', 'Secondary', 'Tertiary', 'Quaternary',
                      'Automatic', 'Customize', 'Edit', 'DESKTOP INTEGRATIONS'):
            self.assertIn(label, source)
        self.assertIn('Use the wallpaper color again', source)
        self.assertNotIn('Primario', source)

    def test_backend_errors_and_tray_messages_are_english(self):
        main = (QML / 'Main.qml').read_text()
        self.assertIn('Pause animations', main)
        self.assertIn('Restore wallpapers', main)
        self.assertIn('Could not read resource usage', main)
        sources = [ROOT / 'app' / 'src' / 'theme_manager.rs',
                   ROOT / 'app' / 'src' / 'settings_controller.rs',
                   ROOT / 'crates' / 'p3w-platform' / 'src' / 'linux' / 'universal_wallpaper.rs']
        for path in sources:
            with self.subTest(file=path.name):
                strings = ' '.join(re.findall(r'"(?:\\.|[^"\\])*"', path.read_text()))
                self.assertIsNone(NON_ENGLISH_WORDS.search(strings))

    def test_public_readme_is_english(self):
        source = (ROOT / 'README.md').read_text()
        self.assertIn('Synchronized settings and editable palette', source)
        self.assertNotRegex(source, '[áéíóúñÁÉÍÓÚÑ¿¡]')

if __name__ == '__main__':
    unittest.main()
