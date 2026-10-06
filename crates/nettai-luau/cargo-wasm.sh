#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

: "${WASI_SDK_PATH:?Set WASI_SDK_PATH to an extracted WASI SDK 34}"
if [ "$#" -eq 0 ]; then
    echo "usage: $0 <build|check|test> [cargo options]" >&2
    exit 2
fi
cargo_command="$1"
shift
case "$cargo_command" in
    build|check|test) ;;
    *) echo "expected build, check, or test" >&2; exit 2 ;;
esac
exec cargo +nightly "$cargo_command" --locked \
    -Zbuild-std=std,panic_unwind \
    -p nettai-luau --target wasm32-unknown-unknown "$@"
