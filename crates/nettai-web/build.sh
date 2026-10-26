#!/usr/bin/env bash
# The nettai app as a page: crates/nettai-web built for the browser with Wasm's atomics (its threads are Web
# Workers on one shared memory), wasm-bindgen's shim, the page, and the content's archive, in one folder.
#
#   bash crates/nettai-web/build.sh [release|dev] [out]     (default: release, target/web/site)
#   python3 crates/nettai-web/serve.py [out]                (serves it as a page with threads must be)
#
# The module is cargo's alone: `cargo +nightly build --target wasm32-unknown-unknown -p nettai-web`, its flags and
# the standard library built again with atomics in .cargo/config.toml. This script makes the page of it.
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
cargo +nightly build --locked --target wasm32-unknown-unknown -p nettai-web --profile "$profile"

mkdir -p "$out"
wasm-bindgen --target web --out-dir "$out" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/$dir/nettai_web.wasm"
cp crates/nettai-web/web/index.html docs/logo/nettai-icon.svg docs/logo/nettai-logotype.svg "$out/"
# The content: content/'s Luau and its tables, without the platform's litter or the submodules' .git files.
tar --exclude .DS_Store --exclude .git -cf "$out/content.tar" -C content .

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
