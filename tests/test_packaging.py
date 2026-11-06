"""Installer/launcher regression checks without requiring Cargo or Qt."""
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
PKG = ROOT / 'packaging'


class PackagingTests(unittest.TestCase):
    def test_desktop_basics(self):
        text = (PKG / 'desktop/persona3-wallpaper.desktop').read_text()
        for field in ('[Desktop Entry]', 'Type=Application', 'Name=Persona 3 Wallpaper',
                      'Exec=persona3-wallpaper', 'Icon=persona3-wallpaper',
                      'Terminal=false', 'Categories='):
            self.assertIn(field, text)
        self.assertNotIn('target/debug', text)
        self.assertNotIn('OnlyShowIn=', text)

    def test_icon_exists(self):
        icon = PKG / 'icons/persona3-wallpaper.png'
        self.assertGreater(icon.stat().st_size, 1000)
        self.assertEqual(icon.read_bytes()[:8], b'\x89PNG\r\n\x1a\n')

    def test_quote_exec(self):
        spec = importlib.util.spec_from_file_location('p3w_desktop', PKG / 'write_desktop.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.assertEqual(module.quote_exec(Path('/tmp/test 100%\"$`/run')),
                         '"/tmp/test 100%%\\\"\\$\\`/run"')
        with self.assertRaises(ValueError):
            module.quote_exec(Path('relative/path'))
        with self.assertRaises(ValueError):
            module.quote_exec(Path('/tmp/bad\nfilename'))

    def test_local_install_uninstall_preserves_data(self):
        with tempfile.TemporaryDirectory(prefix='p3w xdg test ') as temp:
            home = Path(temp) / 'home folder'
            data = Path(temp) / 'data folder'
            config = Path(temp) / 'configuration'
            home.mkdir()
            data.mkdir()
            config.mkdir()
            # Copy a known executable into the sandbox; no Cargo build required.
            exe = shutil.which('true')
            self.assertIsNotNone(exe)
            env = dict(os.environ, HOME=str(home), XDG_DATA_HOME=str(data),
                       XDG_CONFIG_HOME=str(config), P3W_BINARY=exe)
            subprocess.run(['bash', str(ROOT/'scripts/install-local.sh'), '--skip-build'],
                           env=env, check=True, capture_output=True, text=True)
            self.assertTrue((home/'.local/bin/persona3-wallpaper').is_file())
            launcher = data/'applications/persona3-wallpaper.desktop'
            contents = launcher.read_text()
            self.assertIn('Name=Persona 3 Wallpaper', contents)
            self.assertIn('Exec="'+str(home/'.local/bin/persona3-wallpaper')+'"', contents)
            self.assertTrue((data/'icons/hicolor/256x256/apps/persona3-wallpaper.png').exists())
            saved = data/'persona3-wallpaper/wallpapers/keep-me.txt'
            saved.parent.mkdir(parents=True)
            saved.write_text('not deleted')
            autostart = config/'autostart/persona3-wallpaper.desktop'
            autostart.parent.mkdir(parents=True)
            autostart.write_text('[Desktop Entry]\nX-P3W-Managed=true\n')
            subprocess.run(['bash', str(ROOT/'scripts/uninstall-local.sh')], env=env,
                           check=True, capture_output=True, text=True)
            self.assertFalse(launcher.exists())
            self.assertFalse((home/'.local/bin/persona3-wallpaper').exists())
            self.assertFalse(autostart.exists())
            self.assertTrue(saved.exists())

    def test_appimage_launcher_registration(self):
        with tempfile.TemporaryDirectory(prefix='p3w appimage test ') as temp:
            home = Path(temp)/'home path'
            data = Path(temp)/'data path'
            home.mkdir()
            data.mkdir()
            source = Path(temp)/'source.AppImage'
            source.write_bytes(b'example appimage bytes')
            env = dict(os.environ, HOME=str(home), XDG_DATA_HOME=str(data))
            subprocess.run(['bash', str(ROOT/'scripts/install-appimage-local.sh'), str(source)],
                           env=env, check=True, capture_output=True, text=True)
            launcher = (data/'applications/persona3-wallpaper.desktop').read_text()
            self.assertIn(str(home/'.local/bin/persona3-wallpaper.AppImage'), launcher)
            self.assertTrue((home/'.local/bin/persona3-wallpaper.AppImage').exists())
            subprocess.run(['bash', str(ROOT/'scripts/uninstall-local.sh')], env=env,
                           check=True, capture_output=True, text=True)
            self.assertFalse((home/'.local/bin/persona3-wallpaper.AppImage').exists())

    def test_system_package_scripts_syntax(self):
        scripts = ['build-release.sh', 'install-local.sh', 'uninstall-local.sh',
                   'build-arch-package.sh', 'build-deb-package.sh', 'build-appimage.sh',
                   'install-appimage-local.sh', 'ensure-build-deps.sh', 'install-package.sh']
        for name in scripts:
            with self.subTest(name=name):
                subprocess.run(['bash', '-n', str(ROOT/'scripts'/name)], check=True)
        subprocess.run(['bash', '-n', str(PKG/'arch/PKGBUILD')], check=True)

    def test_autostart_installed_executable_and_appimage(self):
        src = (ROOT/'app/src/autostart.rs').read_text()
        self.assertIn('std::env::var_os("APPIMAGE")', src)
        self.assertIn('X-P3W-Managed=true', src)
        self.assertIn('stable_executable()', src)


if __name__ == '__main__':
    unittest.main()
