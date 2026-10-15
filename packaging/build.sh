# shellcheck shell=bash disable=SC2034,SC2120
# The parts packaging/macos and packaging/linux share (sourced, not run).

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

# The version: $NETTAI_VERSION (the workflow's, from the tag), else the app crate's.
version=${NETTAI_VERSION:-$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/crates/nettai/Cargo.toml" | head -n 1)}

# A release build of the app for a packaged app (`NETTAI_PACKAGED`: it reads no content but the content it
# ships). With a target triple, for that target. Sets `notices` to the build's third-party notices
# (crates/nettai/build.rs writes them to its OUT_DIR, which cargo's build-script-executed message names), and
# `program` to the program built (its compiler-artifact message's executable).
build_app() { # [target]
    local args=(build --release --locked -p nettai --message-format=json-render-diagnostics --manifest-path "$root/Cargo.toml")
    [ -n "${1:-}" ] && args+=(--target "$1")
    local messages dir
    messages=$(NETTAI_PACKAGED=1 cargo "${args[@]}")
    notices=
    while IFS= read -r dir; do
        [ -f "$dir/licenses.txt" ] && notices=$dir/licenses.txt
    done < <(sed -n 's/.*"reason":"build-script-executed".*"out_dir":"\([^"]*\)".*/\1/p' <<<"$messages")
    program=$(sed -n 's/.*"reason":"compiler-artifact".*"executable":"\([^"]*\)".*/\1/p' <<<"$messages" | tail -n 1)
    [ -n "$notices" ] || { echo "the build named no licenses.txt (crates/nettai/build.rs)" >&2; return 1; }
    [ -n "$program" ] || { echo "the build named no program" >&2; return 1; }
}

# The repository's content/, into <dest-dir>.
copy_content() { # <dest-dir>
    mkdir -p "$1"
    cp -R "$root/content" "$1/"
}
