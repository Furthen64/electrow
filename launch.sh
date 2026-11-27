#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

address='0.0.0.0'
port=8090
no_build=0
open_browser=0

while [ $# -gt 0 ]; do
    case "$1" in
        --address|-a)
            address=$2; shift 2 ;;
        --port|-p)
            port=$2; shift 2 ;;
        --no-build|-n)
            no_build=1; shift ;;
        --open|-o)
            open_browser=1; shift ;;
        *)
            echo "Unknown option: $1" >&2; exit 2 ;;
    esac
done

lan_flag=()
case "$address" in
    127.0.0.1|localhost|::1) ;;
    *) lan_flag=(--allow-lan) ;;
esac

open_flag=()
if [ "$open_browser" -eq 1 ]; then
    open_flag=(--open)
fi

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

"$py" -m backend --host "$address" --port "$port" "${lan_flag[@]}" "${open_flag[@]}"
