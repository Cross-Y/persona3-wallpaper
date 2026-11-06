#!/usr/bin/env python3
"""Write a freedesktop launcher with a safely quoted absolute executable."""
from pathlib import Path
import sys


def quote_exec(path: Path) -> str:
    text = str(path)
    if not path.is_absolute() or any(char in text for char in ('\n', '\r', '\0')):
        raise ValueError('Executable must be an absolute path without control characters')
    # Exec keys are parsed by desktop entry launchers, not by a shell.
    # Escape % field-code marker, then escape characters in a quoted argument.
    text = (text.replace('%', '%%').replace('\\', '\\\\')
            .replace('"', '\\"').replace('`', '\\`').replace('$', '\\$'))
    return '"' + text + '"'


def create_launcher(template: Path, dest: Path, executable: Path) -> None:
    lines = template.read_text(encoding='utf-8').splitlines()
    if not any(line.startswith('Exec=') for line in lines):
        raise ValueError('Desktop template lacks Exec=')
    lines = [('Exec=' + quote_exec(executable)) if line.startswith('Exec=') else line for line in lines]
    dest.write_text('\n'.join(lines) + '\n', encoding='utf-8')


if __name__ == '__main__':
    if len(sys.argv) != 4:
        sys.exit('Usage: write_desktop.py TEMPLATE DEST EXECUTABLE')
    create_launcher(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]))
