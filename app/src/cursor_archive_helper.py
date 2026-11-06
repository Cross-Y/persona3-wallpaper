"""Safely import Xcursor themes into the user's XDG icon theme directory.

Invoked by Rust with `python3 -I -c <embedded source> <archive/file URL> <icons dir>`.
Never uses shell commands, invokes installers, or writes to system directories.
"""

import json
import os
import posixpath
import re
import shutil
import stat
import sys
import tarfile
import tempfile
import urllib.parse
import zipfile
from pathlib import Path, PurePosixPath

MAX_ARCHIVE = 128 * 1024 * 1024
MAX_UNPACKED = 384 * 1024 * 1024
MAX_MEMBER = 64 * 1024 * 1024
MAX_ENTRIES = 4096
MAX_THEMES = 24
CHUNK_SIZE = 1024 * 1024


class ImportErrorSafe(ValueError):
    pass


def safe_parts(name):
    if '\x00' in name or '\\' in name or any(ord(ch) < 32 for ch in name):
        raise ImportErrorSafe('The archive contains an unsafe filename.')
    if name.startswith('/') or re.match(r'^[A-Za-z]:', name):
        raise ImportErrorSafe('Absolute paths are not allowed in cursor archives.')
    parts = [part for part in name.split('/') if part not in ('', '.')]
    if not parts or any(part == '..' or len(part) > 180 for part in parts) or len(parts) > 20:
        raise ImportErrorSafe('The archive contains an unsafe path.')
    if len(name) > 1024:
        raise ImportErrorSafe('The archive contains an overlong filename.')
    return PurePosixPath(*parts)


def link_target(name, target, hardlink=False):
    if '\x00' in target or '\\' in target or target.startswith('/') or re.match(r'^[A-Za-z]:', target):
        raise ImportErrorSafe('Unsafe symbolic link in cursor archive.')
    if any(ord(ch) < 32 for ch in target):
        raise ImportErrorSafe('Unsafe symbolic link in cursor archive.')
    base = '' if hardlink else str(name.parent)
    normalized = posixpath.normpath(posixpath.join(base, target))
    if normalized in ('.', '..') or normalized.startswith('../'):
        raise ImportErrorSafe('Cursor archive contains a link outside its directory.')
    return safe_parts(normalized)


def limited_copy(source, destination, size, budget):
    if size > MAX_MEMBER or size > budget[0]:
        raise ImportErrorSafe('The cursor archive exceeds the extraction size limit.')
    budget[0] -= size
    remaining = size
    with destination.open('xb') as output:
        while remaining:
            data = source.read(min(CHUNK_SIZE, remaining))
            if not data:
                raise ImportErrorSafe('Unexpected end of cursor archive.')
            output.write(data)
            remaining -= len(data)


def extract_archive(archive, destination):
    budget = [MAX_UNPACKED]
    links = {}
    seen = set()
    count = 0

    def begin(name):
        nonlocal count
        count += 1
        if count > MAX_ENTRIES:
            raise ImportErrorSafe('The cursor archive contains too many files.')
        rel = safe_parts(name)
        if rel in seen:
            raise ImportErrorSafe('The cursor archive contains duplicate entries.')
        seen.add(rel)
        path = destination.joinpath(*rel.parts)
        return rel, path

    if zipfile.is_zipfile(archive):
        with zipfile.ZipFile(archive) as content:
            for item in content.infolist():
                rel, path = begin(item.filename)
                file_type = stat.S_IFMT(item.external_attr >> 16)
                if item.is_dir():
                    if file_type not in (0, stat.S_IFDIR):
                        raise ImportErrorSafe('Unsupported archive entry type.')
                    path.mkdir(parents=True, exist_ok=True)
                elif file_type == stat.S_IFLNK:
                    if item.file_size > 4096:
                        raise ImportErrorSafe('Symbolic link target is too long.')
                    with content.open(item) as file:
                        target = file.read(MAX_MEMBER + 1).decode('utf-8', 'strict')
                    links[rel] = link_target(rel, target)
                elif file_type in (0, stat.S_IFREG):
                    path.parent.mkdir(parents=True, exist_ok=True)
                    with content.open(item) as file:
                        limited_copy(file, path, item.file_size, budget)
                else:
                    raise ImportErrorSafe('Device files and special archive entries are not supported.')
    elif tarfile.is_tarfile(archive):
        with tarfile.open(archive, 'r:*') as content:
            for item in content:
                rel, path = begin(item.name)
                if item.isdir():
                    path.mkdir(parents=True, exist_ok=True)
                elif item.issym() or item.islnk():
                    links[rel] = link_target(rel, item.linkname, item.islnk())
                elif item.isfile():
                    path.parent.mkdir(parents=True, exist_ok=True)
                    file = content.extractfile(item)
                    if file is None:
                        raise ImportErrorSafe('Cannot read a file from the cursor archive.')
                    with file:
                        limited_copy(file, path, item.size, budget)
                else:
                    raise ImportErrorSafe('Device files and special archive entries are not supported.')
    else:
        raise ImportErrorSafe('Unsupported archive. Use ZIP, TAR, TAR.GZ, TAR.XZ or TAR.BZ2.')

    # Resolve internal aliases to regular files, not symlinks. This is compatible
    # with Xcursor's file aliases without letting untrusted links escape staging.
    def resolve(rel, visiting):
        if rel in visiting:
            raise ImportErrorSafe('The cursor archive contains a circular link.')
        if rel in links:
            visiting.add(rel)
            resolved = resolve(links[rel], visiting)
            visiting.remove(rel)
            return resolved
        path = destination.joinpath(*rel.parts)
        if not path.is_file() or path.is_symlink():
            raise ImportErrorSafe('A cursor alias points to a missing or unsafe file.')
        return path

    for rel in links:
        source = resolve(rel, set())
        target = destination.joinpath(*rel.parts)
        target.parent.mkdir(parents=True, exist_ok=True)
        if target.exists():
            raise ImportErrorSafe('A cursor alias conflicts with another archive entry.')
        shutil.copyfile(source, target)
        budget[0] -= target.stat().st_size
        if budget[0] < 0:
            raise ImportErrorSafe('The cursor archive exceeds the extraction size limit.')


def theme_name(name):
    cleaned = re.sub(r'[^a-zA-Z0-9._+\-]', '-', name).strip('-.')
    if not cleaned or cleaned in ('default', 'hicolor') or len(cleaned) > 90:
        raise ImportErrorSafe('The cursor archive has an invalid or reserved theme name.')
    return cleaned


def has_xcursor_data(path):
    with path.open('rb') as file:
        return file.read(4) == b'Xcur'


def find_themes(unpacked, archive):
    directories = sorted((p for p in unpacked.rglob('cursors') if p.is_dir()), key=str)
    if not directories:
        raise ImportErrorSafe('No Xcursor theme found (expected a cursors/ directory).')
    themes = []
    names = set()
    for cursors in directories:
        # The cursor folder must contain actual Xcursor-format data, not just icons.
        files = [p for p in cursors.rglob('*') if p.is_file() and not p.is_symlink()]
        if not files or not any(has_xcursor_data(p) for p in files):
            continue
        root = cursors.parent
        label = root.name if root != unpacked else re.sub(
            r'(?i)\.(zip|tar(?:\.(?:gz|xz|bz2))?|tgz|txz|tbz2?)$', '', archive.name)
        name = theme_name(label)
        if name in names:
            raise ImportErrorSafe('Two themes in this archive have the same name.')
        names.add(name)
        themes.append((name, root))
        if len(themes) > MAX_THEMES:
            raise ImportErrorSafe('The archive contains too many cursor themes.')
    if not themes:
        raise ImportErrorSafe('No valid Xcursor cursor files were found in the archive.')
    return themes


def parse_archive_path(value):
    if value.startswith('file:'):
        url = urllib.parse.urlsplit(value)
        if url.scheme != 'file' or url.netloc not in ('', 'localhost') or url.query or url.fragment:
            raise ImportErrorSafe('Only local cursor archive files are supported.')
        value = urllib.parse.unquote(url.path)
    archive = Path(value)
    if not archive.is_absolute() or not archive.is_file() or archive.is_symlink():
        raise ImportErrorSafe('Select an existing local archive file.')
    if archive.stat().st_size > MAX_ARCHIVE:
        raise ImportErrorSafe('The archive is too large (128 MiB maximum).')
    return archive


def occupied_theme(name, icons_root):
    """Avoid shadowing themes installed in another standard icon directory."""
    search_roots = [icons_root, Path.home() / '.icons']
    xdg_dirs = os.environ.get('XDG_DATA_DIRS', '/usr/local/share:/usr/share')
    for base in xdg_dirs.split(':'):
        if base and os.path.isabs(base):
            search_roots.append(Path(base) / 'icons')
    return any((base / name).exists() or (base / name).is_symlink() for base in search_roots)


def import_themes(value, icons_root):
    archive = parse_archive_path(value)
    icons_root = Path(icons_root)
    if not icons_root.is_absolute():
        raise ImportErrorSafe('Invalid XDG icons directory.')
    icons_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.p3w-cursor-', dir=icons_root) as temp:
        stage = Path(temp)
        unpacked = stage / 'unpacked'
        unpacked.mkdir()
        extract_archive(archive, unpacked)
        themes = find_themes(unpacked, archive)
        for name, _ in themes:
            if occupied_theme(name, icons_root):
                raise ImportErrorSafe(f'Cursor theme "{name}" is already installed. Nothing was overwritten.')
        prepared = stage / 'prepared'
        prepared.mkdir()
        for name, theme_root in themes:
            target = prepared / name
            target.mkdir()
            shutil.copytree(theme_root / 'cursors', target / 'cursors')
            index = theme_root / 'index.theme'
            if index.is_file():
                shutil.copyfile(index, target / 'index.theme')
            else:
                (target / 'index.theme').write_text(f'[Icon Theme]\nName={name}\nComment=Imported cursor theme\n', encoding='utf-8')
        installed = []
        try:
            for name, _ in themes:
                destination = icons_root / name
                if destination.exists() or destination.is_symlink():
                    raise ImportErrorSafe(f'Cursor theme "{name}" is already installed.')
                (prepared / name).rename(destination)
                installed.append(name)
        except Exception:
            for name in installed:
                shutil.rmtree(icons_root / name)
            raise
        return installed


def main():
    try:
        if len(sys.argv) != 3:
            raise ImportErrorSafe('Invalid cursor importer arguments.')
        names = import_themes(sys.argv[1], sys.argv[2])
        print(json.dumps({'ok': True, 'themes': names}))
    except (ImportErrorSafe, OSError, ValueError, tarfile.TarError, zipfile.BadZipFile, RuntimeError, UnicodeError) as error:
        print(json.dumps({'ok': False, 'error': str(error)}))
        sys.exit(1)


if __name__ == '__main__':
    main()
