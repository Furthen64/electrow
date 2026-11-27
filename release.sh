#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$root"

if [ -n "$(git status --porcelain)" ]; then
    echo "warning: working tree has uncommitted changes; archive reflects the working tree" >&2
fi

version="$(awk -F'"' '/pub const ENGINE_VERSION/{print $2; exit}' engine/src/lib.rs)"
if command -v python3 >/dev/null 2>&1; then
    py=python3
else
    py=python
fi
out="$root/Temp/release-$version"
clean="$root/Temp/clean-env-$version"
archive="$out/electrow-$version-source.tar.gz"

mkdir -p "$out"
rm -rf "$clean"
mkdir -p "$clean"

./checkreqs.sh
cargo test --workspace --offline
cargo build --workspace --release --offline
"$py" -m compileall -q backend tests

node --version >/dev/null
for file in web/app.js web/js/*.js; do
    node --check "$file"
done

"$py" tests/smoke.py
"$py" tests/test_api.py
./tests/run_browser_ui.sh

file_list="$(mktemp)"
trap 'rm -f "$file_list"' EXIT
git ls-files --cached --others --exclude-standard |
    grep -v '^FutureSimulation/' |
    while IFS= read -r file; do
        if [ -e "$file" ]; then
            printf '%s\n' "$file"
        fi
    done > "$file_list"
tar -czf "$archive" -T "$file_list"

if [ -f target/release/electrow-engine.exe ]; then
    cp target/release/electrow-engine.exe "$out/"
else
    cp target/release/electrow-engine "$out/"
fi

tar -xzf "$archive" -C "$clean"
(
    cd "$clean"
    cargo test --workspace --offline
    cargo build --workspace --offline
    "$py" tests/smoke.py
    "$py" tests/test_api.py
)

checksums="$(mktemp)"
(
    cd "$out"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum electrow-* > "$checksums"
    else
        shasum -a 256 electrow-* > "$checksums"
    fi
)
mv "$checksums" "$out/SHA256SUMS"

echo "Release artifacts in $out"
cat "$out/SHA256SUMS"
