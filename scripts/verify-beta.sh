#!/usr/bin/env bash
# Source regression checks and native build validation on the user's Linux machine.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
echo "== Python regression suite =="
python3 -m unittest discover -v -s tests
echo
if ! command -v cargo >/dev/null 2>&1; then
    echo "Cargo is not installed or not in PATH. Install Rust, then run this script again." >&2
    exit 1
fi
echo "== Cargo workspace check =="
cargo check --workspace
echo "== Rust unit tests =="
cargo test --workspace
echo
printf '%s\n' "Automated source/build checks passed. A real desktop graphical test is still required."
