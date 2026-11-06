"""Offline package manager simulations: no real sudo/apt/pacman calls."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / 'scripts' / 'ensure-build-deps.sh'
INSTALLER = ROOT / 'scripts' / 'install-package.sh'


class DependencyInstallerTests(unittest.TestCase):
    def prepare(self, temp, distro):
        base = Path(temp)
        bin_dir = base / 'bin'
        bin_dir.mkdir()
        (base / 'os-release').write_text(f'ID={distro}\nPRETTY_NAME="Test distro"\n')
        fake_installed = base / 'installed.txt'
        fake_installed.write_text('')
        # --check must not call this fake sudo. --yes may only write sandbox files.
        scripts = {
            'id': '#!/bin/sh\nif [ "$1" = "-u" ]; then echo 1000; else /usr/bin/id "$@"; fi\n',
            'pacman': '''#!/bin/sh
if [ "$1" = '-Qq' ]; then
    grep -Fxq -- "$3" "$P3W_FAKE_INSTALLED"
else
    exit 20
fi
''',
            'dpkg-query': '''#!/bin/sh
for pkg; do :; done
if grep -Fxq -- "$pkg" "$P3W_FAKE_INSTALLED"; then
    printf 'install ok installed'
else
    exit 1
fi
''',
            'apt-get': '#!/bin/sh\nexit 0\n',
            'rpm': '''#!/bin/sh
for pkg; do :; done
grep -Fxq -- "$pkg" "$P3W_FAKE_INSTALLED"
''',
            'dnf': '#!/bin/sh\nexit 0\n',
            'sudo': '''#!/bin/sh
printf '%s\\n' "$*" >> "$P3W_FAKE_COMMANDS"
case "$1" in
  pacman|apt-get|dnf)
    while [ "$#" -gt 0 ]; do
      if [ "$1" = '--' ]; then
        shift
        printf '%s\\n' "$@" >> "$P3W_FAKE_INSTALLED"
        exit 0
      fi
      shift
    done
    ;;
  apt)
    exit 0
    ;;
esac
exit 20
''',
        }
        for name, content in scripts.items():
            file = bin_dir / name
            file.write_text(content)
            file.chmod(0o755)
        env = dict(os.environ)
        env.update(P3W_OS_RELEASE_FILE=str(base/'os-release'), P3W_FAKE_INSTALLED=str(fake_installed),
                   P3W_FAKE_COMMANDS=str(base/'commands.txt'), PATH=f'{bin_dir}:{os.environ.get("PATH", "")}')
        return env

    def test_arch_check_only(self):
        with tempfile.TemporaryDirectory() as temp:
            env = self.prepare(temp, 'arch')
            p = subprocess.run(['bash', str(CHECKER), '--check'], env=env, capture_output=True, text=True)
            self.assertEqual(p.returncode, 2, p.stderr)
            self.assertIn('qt6-declarative', p.stdout)
            self.assertIn('sudo pacman -S --needed', p.stdout)
            self.assertFalse(Path(env['P3W_FAKE_COMMANDS']).exists())

    def test_arch_explicit_permission_installs_only_missing(self):
        with tempfile.TemporaryDirectory() as temp:
            env = self.prepare(temp, 'cachyos')
            Path(env['P3W_FAKE_INSTALLED']).write_text('qt6-base\nffmpeg\n')
            p = subprocess.run(['bash', str(CHECKER), '--yes'], env=env, capture_output=True, text=True)
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
            self.assertIn('All required', subprocess.run(['bash', str(CHECKER), '--check'],
                          env=env, capture_output=True, text=True).stdout)
            self.assertNotIn('qt6-base ', Path(env['P3W_FAKE_COMMANDS']).read_text())

    def test_debian_check_and_install(self):
        with tempfile.TemporaryDirectory() as temp:
            env = self.prepare(temp, 'ubuntu')
            p = subprocess.run(['bash', str(CHECKER), '--check'], env=env, capture_output=True, text=True)
            self.assertEqual(p.returncode, 2)
            self.assertIn('qml6-module-qtquick-dialogs', p.stdout)
            self.assertIn('sudo apt-get install --', p.stdout)
            p = subprocess.run(['bash', str(CHECKER), '--yes'], env=env, capture_output=True, text=True)
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
            self.assertIn('apt-get install --', Path(env['P3W_FAKE_COMMANDS']).read_text())

    def test_fedora_check(self):
        with tempfile.TemporaryDirectory() as temp:
            env = self.prepare(temp, 'fedora')
            p = subprocess.run(['bash', str(CHECKER), '--check'], env=env, capture_output=True, text=True)
            self.assertEqual(p.returncode, 2)
            self.assertIn('qt6-qtmultimedia-devel', p.stdout)
            self.assertIn('ffmpeg-free', p.stdout)

    def test_package_installer_sends_file_to_manager(self):
        with tempfile.TemporaryDirectory() as temp:
            env = self.prepare(temp, 'ubuntu')
            pkg = Path(temp) / 'persona3 wallpaper.deb'
            pkg.write_bytes(b'test pkg only')
            p = subprocess.run(['bash', str(INSTALLER), str(pkg)], env=env, capture_output=True, text=True)
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
            self.assertIn(str(pkg), Path(env['P3W_FAKE_COMMANDS']).read_text())
            self.assertIn('apt install', Path(env['P3W_FAKE_COMMANDS']).read_text())

    def test_deb_includes_qt_dialogs_runtime(self):
        deb_script = (ROOT/'scripts/build-deb-package.sh').read_text()
        self.assertIn('qml6-module-qtquick-dialogs', deb_script)

    def test_install_local_preflight_and_skip_build(self):
        script = (ROOT/'scripts/install-local.sh').read_text()
        self.assertIn('"$ROOT/scripts/ensure-build-deps.sh"', script)
        self.assertLess(script.index('"$ROOT/scripts/ensure-build-deps.sh"'),
                        script.index('"$ROOT/scripts/build-release.sh"'))


if __name__ == '__main__':
    unittest.main()
