#!/usr/bin/env bash
set -u

failed=0

check_result() {
    local ok=$1 label=$2 detail=$3
    if [ "$ok" -eq 1 ]; then
        printf '[OK  ] %s - %s\n' "$label" "$detail"
    else
        printf '[FAIL] %s - %s\n' "$label" "$detail"
        failed=$((failed + 1))
    fi
}

cargo_version=$(cargo --version 2>&1) || cargo_version=

if [ -z "$cargo_version" ]; then
    check_result 0 cargo 'not found on PATH'
elif [[ "$cargo_version" =~ ^cargo\ ([0-9]+\.[0-9]+\.[0-9]+) ]]; then
    check_result 1 cargo "$cargo_version"
else
    check_result 0 cargo "unexpected version output: $cargo_version"
fi

rustc_version=$(rustc --version 2>&1) || rustc_version=

if [ -z "$rustc_version" ]; then
    check_result 0 rustc 'not found on PATH'
elif [[ "$rustc_version" =~ ^rustc\ ([0-9]+\.[0-9]+\.[0-9]+) ]]; then
    check_result 1 rustc "$rustc_version"
else
    check_result 0 rustc "unexpected version output: $rustc_version"
fi

python_cmd=

python_version=
if command -v python3 >/dev/null 2>&1; then
    python_cmd=python3
    python_version=$(python3 --version 2>&1)
elif command -v python >/dev/null 2>&1; then
    python_cmd=python
    python_version=$(python --version 2>&1)
fi

if [ -z "$python_version" ]; then
    check_result 0 Python 'python3 or python not found on PATH (need 3.12+)'
elif [[ "$python_version" =~ Python\ ([0-9]+)\.([0-9]+) ]]; then
    major=${BASH_REMATCH[1]}
    minor=${BASH_REMATCH[2]}
    if [ "$major" -gt 3 ] || { [ "$major" -eq 3 ] && [ "$minor" -ge 12 ]; }; then
        check_result 1 Python "$python_version (via $python_cmd; need 3.12+)"
    else
        check_result 0 Python "$python_version (via $python_cmd; need 3.12+)"
    fi
else
    check_result 0 Python "unexpected version output: $python_version"
fi

echo
if [ "$failed" -gt 0 ]; then
    echo "$failed requirement check(s) failed."
    exit 1
fi
echo 'All requirements present.'
exit 0
