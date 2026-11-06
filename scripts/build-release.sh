#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
command -v cargo >/dev/null || { echo 'Rust/Cargo is required for building.' >&2; exit 1; }
cd "$ROOT"
cargo build --locked --release -p persona3-wallpaper
printf '\nRelease binary: %s\n' "$ROOT/target/release/persona3-wallpaper"
