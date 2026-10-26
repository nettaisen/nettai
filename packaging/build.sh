# shellcheck shell=bash disable=SC2034,SC2120
# The parts packaging/macos and packaging/linux share (sourced, not run).

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

# The version: $NETTAI_VERSION if set, else the workspace's (Cargo.toml).
version=${NETTAI_VERSION:-$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)}

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

# The repository's content/, into <dest-dir>, without the submodules' .git files.
copy_content() { # <dest-dir>
    mkdir -p "$1"
    cp -R "$root/content" "$1/"
    find "$1/content" -name .git -prune -exec rm -rf {} +
}

# The ffmpeg every package bundles beside the program, for the replay export (docs/app.md §19): a release of
# nettaisen/ffmpeg-build (a minimal static build of what the export runs), pinned in
# packaging/ffmpeg.sha256 by its tag and each file by its SHA-256. `fetch_ffmpeg <file>` downloads that file of
# the release once into target/package/ffmpeg/<tag>/, checks it, and sets `fetched` to it.
ffmpeg_pins=$root/packaging/ffmpeg.sha256
ffmpeg_release=$(sed -n 's/^# release: //p' "$ffmpeg_pins")
fetch_ffmpeg() { # <file>
    local sum dir
    sum=$(awk -v file="$1" '$2 == file { print $1 }' "$ffmpeg_pins")
    [ -n "$sum" ] || { echo "packaging/ffmpeg.sha256 pins no $1" >&2; return 1; }
    dir=$root/target/package/ffmpeg/$ffmpeg_release
    fetched=$dir/$1
    mkdir -p "$dir"
    if [ "$(sha256_of "$fetched")" != "$sum" ]; then
        curl --fail --location --silent --show-error --output "$fetched" \
            "https://github.com/nettaisen/ffmpeg-build/releases/download/$ffmpeg_release/$1"
        [ "$(sha256_of "$fetched")" = "$sum" ] || { echo "$1 of $ffmpeg_release isn't the one pinned" >&2; rm -f "$fetched"; return 1; }
    fi
}

# A file's SHA-256 (nothing for no file): sha256sum on Linux, shasum on macOS.
sha256_of() { # <file>
    [ -f "$1" ] || return 0
    if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d ' ' -f 1
}
