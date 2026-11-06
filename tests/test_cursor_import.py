"""Regression tests for the cross-distro cursor archive importer."""
import io
import json
import os
import pathlib
import stat
import subprocess
import tarfile
import tempfile
import unittest
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
HELPER = ROOT / 'app/src/cursor_archive_helper.py'
XCUR = b'Xcur' + b'\x00' * 20


class ImportCursorTests(unittest.TestCase):
    def setUp(self):
        self.context = tempfile.TemporaryDirectory()
        self.base = pathlib.Path(self.context.name)
        self.icons = self.base / 'icons'

    def tearDown(self):
        self.context.cleanup()

    def invoke(self, archive, source=None):
        source = source or str(archive)
        result = subprocess.run(['python3', '-I', '-c', HELPER.read_text(), source, str(self.icons)], capture_output=True, text=True)
        self.assertTrue(result.stdout, result.stderr)
        return result.returncode, json.loads(result.stdout)

    def test_zip_nested_directory_install_and_selectable(self):
        file = self.base / 'Cool.zip'
        with zipfile.ZipFile(file, 'w') as zip:
            zip.writestr('cool-cursors/CoolBlue/cursors/left_ptr', XCUR)
            zip.writestr('cool-cursors/CoolBlue/index.theme', '[Icon Theme]\nName=CoolBlue\n')
            zip.writestr('cool-cursors/CoolBlue/installer.sh', 'do not execute')
        code, result = self.invoke(file)
        self.assertEqual((code, result['themes']), (0, ['CoolBlue']))
        self.assertEqual((self.icons / 'CoolBlue/cursors/left_ptr').read_bytes(), XCUR)
        self.assertFalse((self.icons / 'CoolBlue/installer.sh').exists())

    def test_multiple_themes_and_flat_archives(self):
        file = self.base / 'multiple.zip'
        with zipfile.ZipFile(file, 'w') as zip:
            for name in ['Blue', 'Red']:
                zip.writestr(name + '/cursors/left_ptr', XCUR)
        code, result = self.invoke(file)
        self.assertEqual((code, result['themes']), (0, ['Blue', 'Red']))
        self.assertTrue((self.icons / 'Red/index.theme').is_file())
        flat = self.base / 'Solo.zip'
        with zipfile.ZipFile(flat, 'w') as zip:
            zip.writestr('cursors/left_ptr', XCUR)
        code, result = self.invoke(flat)
        self.assertEqual((code, result['themes']), (0, ['Solo']))

    def test_tar_gz_and_xz_and_bz2(self):
        for compression, suffix in [('w:gz', 'gz'), ('w:xz', 'xz'), ('w:bz2', 'bz2')]:
            with self.subTest(compression=compression):
                file = self.base / ('theme.' + suffix + '.tar')
                with tarfile.open(file, compression) as tar:
                    item = tarfile.TarInfo('Theme_' + suffix + '/cursors/left_ptr')
                    item.size = len(XCUR)
                    tar.addfile(item, io.BytesIO(XCUR))
                code, result = self.invoke(file)
                self.assertEqual(code, 0, result)
                self.assertEqual(result['themes'], ['Theme_' + suffix])

    def test_zip_internal_symlink_alias_materialized(self):
        file = self.base / 'Symlink.zip'
        with zipfile.ZipFile(file, 'w') as zip:
            zip.writestr('Alias/cursors/left_ptr', XCUR)
            symlink = zipfile.ZipInfo('Alias/cursors/hand1')
            symlink.create_system = 3
            symlink.external_attr = (stat.S_IFLNK | 0o777) << 16
            zip.writestr(symlink, 'left_ptr')
        code, result = self.invoke(file)
        self.assertEqual(code, 0, result)
        alias = self.icons / 'Alias/cursors/hand1'
        self.assertTrue(alias.is_file())
        self.assertFalse(alias.is_symlink())
        self.assertEqual(alias.read_bytes(), XCUR)

    def test_tar_internal_symlink_alias_materialized(self):
        file = self.base / 'Alias.tar'
        with tarfile.open(file, 'w') as tar:
            item = tarfile.TarInfo('AliasTar/cursors/left_ptr'); item.size = len(XCUR)
            tar.addfile(item, io.BytesIO(XCUR))
            alias = tarfile.TarInfo('AliasTar/cursors/hand1')
            alias.type = tarfile.SYMTYPE; alias.linkname = 'left_ptr'
            tar.addfile(alias)
        code, result = self.invoke(file)
        self.assertEqual(code, 0, result)
        self.assertEqual((self.icons / 'AliasTar/cursors/hand1').read_bytes(), XCUR)

    def test_preexisting_theme_never_overwritten(self):
        target = self.icons / 'Already/cursors'
        target.mkdir(parents=True)
        (target / 'left_ptr').write_bytes(b'keep this')
        file = self.base / 'duplicate.zip'
        with zipfile.ZipFile(file, 'w') as zip:
            zip.writestr('Already/cursors/left_ptr', XCUR)
        code, result = self.invoke(file)
        self.assertNotEqual(code, 0)
        self.assertIn('already installed', result['error'])
        self.assertEqual((target / 'left_ptr').read_bytes(), b'keep this')

    def test_absolute_and_parent_traversal_rejected_without_side_effects(self):
        for name in ['../../escape.txt', '/tmp/escape.txt', 'Bad/../Good/cursors/left_ptr']:
            with self.subTest(name=name):
                file = self.base / 'evil.zip'
                with zipfile.ZipFile(file, 'w') as zip:
                    zip.writestr('Good/cursors/left_ptr', XCUR)
                    zip.writestr(name, b'bad')
                code, result = self.invoke(file)
                self.assertNotEqual(code, 0)
                self.assertFalse((self.icons / 'Good').exists())
                self.assertIn('archive', result['error'].lower())
        self.assertFalse((self.base / 'escape.txt').exists())

    def test_external_symlink_and_device_file_rejected(self):
        file = self.base / 'badlink.tar'
        with tarfile.open(file, 'w') as tar:
            item = tarfile.TarInfo('Safe/cursors/left_ptr'); item.size = len(XCUR)
            tar.addfile(item, io.BytesIO(XCUR))
            link = tarfile.TarInfo('Safe/cursors/escape'); link.type = tarfile.SYMTYPE; link.linkname = '../../../../etc/passwd'
            tar.addfile(link)
        code, result = self.invoke(file)
        self.assertNotEqual(code, 0)
        self.assertFalse((self.icons / 'Safe').exists())

    def test_rejects_non_cursor_theme(self):
        file = self.base / 'not-cursor.zip'
        with zipfile.ZipFile(file, 'w') as zip:
            zip.writestr('Pretty/cursors/pointer.png', b'PNG bad')
        code, result = self.invoke(file)
        self.assertNotEqual(code, 0)
        self.assertIn('No valid Xcursor', result['error'])

    def test_file_url_is_supported(self):
        file = self.base / 'url test.zip'
        with zipfile.ZipFile(file, 'w') as zip:
            zip.writestr('UrlTheme/cursors/left_ptr', XCUR)
        code, result = self.invoke(file, file.as_uri())
        self.assertEqual((code, result['themes']), (0, ['UrlTheme']))

    def test_rejects_remote_file_url(self):
        code, result = self.invoke(None, 'file://remotehost/other.zip')
        self.assertNotEqual(code, 0)
        self.assertIn('local', result['error'].lower())

    def test_ui_bindings_and_rust_bridge(self):
        qml = (ROOT / 'app/qml/views/AppearanceView.qml').read_text()
        bridge = (ROOT / 'app/src/settings_controller.rs').read_text()
        manager = (ROOT / 'app/src/cursor_manager.rs').read_text()
        self.assertIn('import QtQuick.Dialogs', qml)
        self.assertIn('cursorImportDialog.open()', qml)
        self.assertIn('onAccepted: root.importCursorArchive(String(selectedFile))', qml)
        self.assertIn('fn import_cursor_archive(self: Pin<&mut SettingsController>, archive_url: QString) -> QString;', bridge)
        self.assertIn('include_str!("cursor_archive_helper.py")', manager)


if __name__ == '__main__':
    unittest.main()
