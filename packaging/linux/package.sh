#!/usr/bin/env bash
# Velopack's release of the app, on Linux, for one architecture: target/package/linux/<arch>/releases/
# (nettai-<channel>.AppImage, the app; and the update package and its feed the installed app's updater reads from the
# GitHub release: nettai-<version>-<channel>-full.nupkg, releases.<channel>.json, assets.<channel>.json,
# RELEASES-<channel>).
#
#   NETTAI_LINUX_ARCH=arm64 packaging/linux/package.sh
#
# Each architecture is its own package on its own Velopack channel, named for the system and the architecture as
# every system's is (which is also Velopack's runtime, `vpk pack --runtime`), so that each one's files and feed sit
# beside the other's in one release. Each is built natively, on a machine of its architecture:
#
#   x86_64     linux-x64      nettai-linux-x64.AppImage
#   arm64      linux-arm64    nettai-linux-arm64.AppImage
#
# (Up to 0.19.1 x86_64's app was packed a second time on the channel `linux`, the only one up to 0.19.0: the bridge,
# docs/app.md §19, "Updates".)
#
# The app's folder, which vpk makes the AppImage of: nettai and ffmpeg (beside the program, where the app looks for
# it), and LICENSE, THIRD-PARTY-NOTICES.txt and ffmpeg-NOTICE.txt. vpk writes the AppImage's entry, its desktop file
# and its icon. No content, no ROM and no extracted asset is in it: the app downloads each game's content
# (docs/design/content-deployment.md §5) and extracts each game's pack from the player's own copy. The system's
# libraries (glibc, ALSA, fontconfig, OpenGL, X11 or Wayland) are the system's: build it on an old distribution (the
# workflow's is Ubuntu 22.04, on each architecture) to reach more.
#
# NETTAI_LINUX_ARCH      x86_64 or arm64 (above): this machine's own, which is the default
# NETTAI_VERSION         the version, if set; else the app crate's
# NETTAI_ICONS           a folder packaging/icons.sh made; else it is made here (needs librsvg and ImageMagick)
# NETTAI_BUNDLE_FFMPEG   the ffmpeg to bundle (for the architecture packaged); else the release
#                        packaging/ffmpeg.sha256 pins (a static program), downloaded and checked
# VPK                    Velopack's vpk (the version crates/nettai's velopack is: 1.3.39-g8d2844f); else vpk on the PATH
set -euo pipefail
# shellcheck source-path=SCRIPTDIR source=../build.sh
source "$(dirname "$0")/../build.sh"

host=$(uname -m)
host=${host/aarch64/arm64}
arch=${NETTAI_LINUX_ARCH:-$host}
# The channel, which is also Velopack's runtime.
case $arch in
    x86_64) channel=linux-x64 ;;
    arm64) channel=linux-arm64 ;;
    *) echo "NETTAI_LINUX_ARCH is x86_64 or arm64, not $arch" >&2; exit 1 ;;
esac
[ "$arch" = "$host" ] || { echo "the $arch package is built on $arch Linux, not on this $host machine" >&2; exit 1; }

out=$root/target/package/linux/$arch
rm -rf "$out"
mkdir -p "$out"

build_app ""

icons=${NETTAI_ICONS:-}
if [ -z "$icons" ]; then
    icons=$out/icons
    "$root/packaging/icons.sh" "$icons"
fi

app=$out/nettai
mkdir -p "$app"
# (The release profile keeps line tables for the log's backtraces; the program's symbols stay, its debug info goes.)
cp "$program" "$app/nettai"
strip --strip-debug "$app/nettai"
cp "$root/LICENSE" "$app/LICENSE"
cp "$notices" "$app/THIRD-PARTY-NOTICES.txt"
# ffmpeg beside the program, where the export looks first (nettai_video::mp4::find_ffmpeg): the release's program for
# the architecture; and its notice (GPL: what it is built from, where that source is).
if [ -n "${NETTAI_BUNDLE_FFMPEG:-}" ]; then
    cp "$NETTAI_BUNDLE_FFMPEG" "$app/ffmpeg"
else
    fetch_ffmpeg "ffmpeg-linux-$arch"
    cp "$fetched" "$app/ffmpeg"
fi
chmod 755 "$app/ffmpeg"
fetch_ffmpeg ffmpeg-NOTICE.txt
cp "$fetched" "$app/ffmpeg-NOTICE.txt"

# The release (Velopack's: docs/app.md §19), on the architecture's channel.
"${VPK:-vpk}" pack --packId nettai --packVersion "$version" --packTitle nettai --packAuthors nettai \
    --packDir "$app" --mainExe nettai --channel "$channel" --runtime "$channel" --icon "$icons/png/256x256.png" \
    --categories "Game;ActionGame;" --outputDir "$out/releases"

echo "$out/releases"
