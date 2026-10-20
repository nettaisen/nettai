#!/usr/bin/env bash
# The nettai app as a page: crates/nettai-web built for the browser with Wasm's atomics (its threads are Web
# Workers on one shared memory), wasm-bindgen's shim, the page, and the content's archive, in one folder.
#
#   bash crates/nettai-web/build.sh [release|dev] [out]     (default: release, target/web/site)
#   python3 crates/nettai-web/serve.py [out]                (serves it as a page with threads must be)
#
# Needs: Rust nightly with rust-src (the standard library is built again with atomics), the wasm32-unknown-unknown
# target, wasm-bindgen-cli at Cargo.lock's wasm-bindgen version, and WASI SDK 34 at $WASI_SDK_PATH (Luau's C++:
# crates/nettai-luau/README.md).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."

: "${WASI_SDK_PATH:?Set WASI_SDK_PATH to an extracted WASI SDK 34 (crates/nettai-luau/README.md)}"
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

# Atomics and a shared memory the page makes (imported), as large as Wasm's 32-bit memory may be; the thread-local
# storage's symbols that wasm-bindgen sets up each thread with. (Its own target folder: these flags make every crate
# another build.)
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/web}"
export RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals \
-C link-arg=--shared-memory -C link-arg=--import-memory -C link-arg=--max-memory=4294967296 \
-C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size -C link-arg=--export=__tls_align \
-C link-arg=--export=__tls_base -C link-arg=--export=__heap_base"
cargo +nightly build --locked -Zbuild-std=std,panic_abort --target wasm32-unknown-unknown -p nettai-web --profile "$profile"

mkdir -p "$out"
wasm-bindgen --target web --out-dir "$out" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/$dir/nettai_web.wasm"
cp crates/nettai-web/web/index.html docs/logo/nettai-icon.svg docs/logo/nettai-logotype.svg "$out/"
# The content: content/'s Luau and its tables, without the platform's litter.
tar --exclude .DS_Store -cf "$out/content.tar" -C content .

# A release's module optimized and without its names and debug information (a dev build keeps its function names,
# for the console's stack traces): binaryen's wasm-opt, with the features the build has.
if [ "$profile" = release ]; then
    if command -v wasm-opt >/dev/null; then
        wasm-opt -O2 --enable-threads --enable-bulk-memory --enable-mutable-globals --enable-exception-handling \
            --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types --enable-multivalue \
            --strip-debug --strip-producers -o "$out/nettai_web_bg.wasm" "$out/nettai_web_bg.wasm"
    else
        echo "no wasm-opt (binaryen: brew install binaryen): the module is left as it was built" >&2
    fi
fi

# The sizes the page's preload screen counts its downloads against (as decoded: a server may send them compressed).
printf '{"nettai_web_bg.wasm": %s, "content.tar": %s}\n' "$(wc -c < "$out/nettai_web_bg.wasm" | tr -d ' ')" "$(wc -c < "$out/content.tar" | tr -d ' ')" > "$out/sizes.json"

# Each file compressed beside it, for a server that sends what the browser takes (serve.py does): brotli where
# there is one, gzip.
for file in "$out"/*.wasm "$out"/*.js "$out"/content.tar; do
    rm -f "$file.br" "$file.gz"
    if command -v brotli >/dev/null; then brotli -q 9 -f -o "$file.br" "$file"; fi
    gzip -9 -k -f "$file"
done
echo "built $out: python3 crates/nettai-web/serve.py $out"
