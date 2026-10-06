#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

: "${WASI_SDK_PATH:?Set WASI_SDK_PATH to an extracted WASI SDK 34}"
build_profile="${1:-debug}"
profile_args=()
case "$build_profile" in
    debug) ;;
    release) profile_args=(--release) ;;
    *) echo "usage: $0 [debug|release]" >&2; exit 2 ;;
esac

bash crates/nettai-luau/cargo-wasm.sh test --lib "${profile_args[@]}"
bash crates/nettai-luau/cargo-wasm.sh build --example wasm_smoke "${profile_args[@]}"
node crates/nettai-luau/tests/wasm-smoke.mjs \
    "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/$build_profile/examples/wasm_smoke.wasm"
