#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

address='127.0.0.1'
port=8000
no_build=0

while [ $# -gt 0 ]; do
    case "$1" in
        --address|-a)
            address=$2; shift 2 ;;
        --port|-p)
            port=$2; shift 2 ;;
        --no-build|-n)
            no_build=1; shift ;;
        *)
            echo "Unknown option: $1" >&2; exit 2 ;;
    esac
done

if [ "$no_build" -eq 0 ]; then
    "$root/build.sh"
fi

cd "$root"
if command -v python3 >/dev/null 2>&1; then
    py=python3
elif command -v python >/dev/null 2>&1; then
    py=python
else
    echo 'python3/python not found' >&2; exit 1
fi

"$py" -m backend --host "$address" --port "$port"
