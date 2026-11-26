#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"$root/checkreqs.sh"

cd "$root"
cargo build --workspace

if command -v python3 >/dev/null 2>&1; then
    py=python3
elif command -v python >/dev/null 2>&1; then
    py=python
else
    echo 'python3/python not found; skipping Python syntax check' >&2
    exit 1
fi

"$py" -m compileall -q backend tests

if command -v node >/dev/null 2>&1; then
    node --check web/app.js
else
    echo 'node not found; skipping JavaScript syntax check'
fi

echo 'Build OK.'
