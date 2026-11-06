# Linux packaging

The app has an XDG desktop launcher and separate package builders for:

- Per-user release install (Arch, Ubuntu, Fedora with compatible runtime Qt):
  `bash scripts/install-local.sh` (prompts to install missing source-build packages)
- Arch/CachyOS: `bash scripts/build-arch-package.sh`
- Debian/Ubuntu, built on target distro: `bash scripts/build-deb-package.sh`
- AppImage (experimental, requires linuxdeploy Qt plugin):
  `bash scripts/build-appimage.sh`

See [`docs/INSTALL_AND_PACKAGING.md`](../docs/INSTALL_AND_PACKAGING.md).

**Do not release the bundled Persona artwork or third-party game audio unless
properly licensed.** The icon in `packaging/icons` is derived from the
existing app tray image strictly for local beta testing. Replace it before
public distribution if the original is not licensed for redistribution.

The source-code license text is in `../LICENSE`. Multimedia assets require their own permissions.

For prebuilt packages, use `bash scripts/install-package.sh /path/to/file.deb`
(or `.pkg.tar.zst`, `.AppImage`). APT and pacman resolve declared dependencies;
no Rust toolchain is installed on the beta tester's machine.
