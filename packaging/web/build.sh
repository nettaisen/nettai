#!/usr/bin/env bash
# The nettai app as a page: the app's crate's library built for the browser as a cdylib, with Wasm's atomics (its
# threads are Web Workers on one shared memory), wasm-bindgen's shim and the page, in one folder. The page ships no
# content: the app downloads each game's from the game's site, as the desktop app does
# (docs/design/content-deployment.md §5).
#
#   bash packaging/web/build.sh [release|dev] [out]     (default: release, target/web/site)
#   python3 packaging/web/serve.py [out]                (serves it as a page with threads must be)
#
# The module is cargo's alone: `cargo +nightly rustc --target wasm32-unknown-unknown -p nettai --lib --crate-type
# cdylib` (what the page calls: crates/nettai/src/page/platform/web/entry.rs), its flags and the standard library
# built again with atomics in .cargo/config.toml. This script makes the page of it.
#
# Needs: Rust nightly with rust-src, the wasm32-unknown-unknown target, wasm-bindgen-cli at Cargo.lock's
# wasm-bindgen version, and WASI SDK 34 at WASI_SDK_PATH, in the environment or ~/.cargo/config.toml's [env]
# (Luau's C++: crates/nettai-luau/README.md).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

profile="${1:-release}"
out="${2:-target/web/site}"
case "$profile" in
    release) dir=release ;;
    dev) dir=debug ;;
    *) echo "usage: $0 [release|dev] [out]" >&2; exit 2 ;;
esac

want=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
have=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}')
if [ "$want" != "$have" ]; then
    echo "wasm-bindgen-cli $want is needed (found ${have:-none}): cargo install wasm-bindgen-cli --version $want --locked" >&2
    exit 1
fi

CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}"
cargo +nightly rustc --locked --target wasm32-unknown-unknown -p nettai --lib --crate-type cdylib --profile "$profile"

mkdir -p "$out"
wasm-bindgen --target web --out-dir "$out" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/$dir/nettai.wasm"
cp packaging/web/index.html docs/logo/nettai-icon.svg docs/logo/nettai-logotype.svg "$out/"

# A release's module optimized and without its names and debug information (a dev build keeps its function names,
# for the console's stack traces): binaryen's wasm-opt, with the features the build has.
if [ "$profile" = release ]; then
    if command -v wasm-opt >/dev/null; then
        wasm-opt -O2 --enable-threads --enable-bulk-memory --enable-mutable-globals --enable-exception-handling \
            --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types --enable-multivalue \
            --strip-debug --strip-producers -o "$out/nettai_bg.wasm" "$out/nettai_bg.wasm"
    else
        echo "no wasm-opt (binaryen: brew install binaryen): the module is left as it was built" >&2
    fi
fi

# The sizes the page's preload screen counts its downloads against (as decoded: a server may send them compressed).
printf '{"nettai_bg.wasm": %s}\n' "$(wc -c < "$out/nettai_bg.wasm" | tr -d ' ')" > "$out/sizes.json"

# Each file compressed beside it, for a server that sends what the browser takes (serve.py does): brotli where
# there is one, gzip.
for file in "$out"/*.wasm "$out"/*.js; do
    rm -f "$file.br" "$file.gz"
    if command -v brotli >/dev/null; then brotli -q 9 -f -o "$file.br" "$file"; fi
    gzip -9 -k -f "$file"
done
echo "built $out: python3 packaging/web/serve.py $out"
