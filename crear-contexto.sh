
#!/usr/bin/env bash

set -e

PROJECT="$(pwd)"
NAME="$(basename "$PROJECT")"
OUTPUT="$HOME/Descargas/${NAME}-contexto.zip"
TEMP="$(mktemp -d)"

echo "Preparing context for $NAME..."

# Project structure
{
    echo "# PROJECT STRUCTURE"
    find . \
        -not -path './.git*' \
        -not -path './target*' \
        -not -path './node_modules*' \
        -not -path './.idea*' \
        -not -path './.vscode*' \
        -type f | sort

    echo
    echo "# GIT STATUS"

    if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        git status --short
    else
        echo "Git not initialized"
    fi

    echo
    echo "# VERSIONS"

    rustc --version 2>/dev/null || true
    cargo --version 2>/dev/null || true

} > "$TEMP/CONTEXTO.md"

# Copy project excluding large artifacts
cd "$PROJECT"

zip -qr "$OUTPUT" . \
    -x '.git/*' \
       '*/.git/*' \
       'target/*' \
       '*/target/*' \
       'node_modules/*' \
       '*/node_modules/*' \
       '.idea/*' \
       '.vscode/*' \
       '*.mp4' \
       '*.mkv' \
       '*.webm' \
       '*.iso' \
       '*.o' \
       '*.so' \
       '*.rlib' \
       '*.rmeta' \
       '*.lock.tmp'

# Add context report
zip -q "$OUTPUT" "$TEMP/CONTEXTO.md"

rm -rf "$TEMP"

echo
echo "Context archive created!"
echo "$OUTPUT"
