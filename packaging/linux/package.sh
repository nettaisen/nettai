#!/usr/bin/env bash
# Velopack's release of the app, on x86_64 Linux: target/package/linux/releases/ (nettai.AppImage, the app; and the
# update package and its feed the installed app's updater reads from the GitHub release:
# nettai-<version>-linux-full.nupkg, releases.linux.json, assets.linux.json, RELEASES-linux).
#
#   packaging/linux/package.sh
#
# The app's folder, which vpk makes the AppImage of: nettai and ffmpeg (beside the program, where the app looks for
# it), and LICENSE, THIRD-PARTY-NOTICES.txt and ffmpeg-NOTICE.txt. vpk writes the AppImage's entry, its desktop file
# and its icon. No content, no ROM and no extracted asset is in it: the app downloads each game's content
# (docs/design/content-deployment.md §5) and extracts each game's pack from the player's own copy. The system's
# libraries (glibc, ALSA, fontconfig, OpenGL, X11 or Wayland) are the system's: build it on an old distribution (the
# workflow's is Ubuntu 22.04) to reach more.
#
# NETTAI_VERSION   the version, if set; else the app crate's
# NETTAI_ICONS     a folder packaging/icons.sh made; else it is made here (needs librsvg and ImageMagick)
# NETTAI_BUNDLE_FFMPEG   the ffmpeg to bundle; else the release packaging/ffmpeg.sha256 pins (a static program),
#                  downloaded and checked
# VPK              Velopack's vpk (the version crates/nettai's velopack is: 1.3.39-g8d2844f); else vpk on the PATH
set -euo pipefail
# shellcheck source-path=SCRIPTDIR source=../build.sh
source "$(dirname "$0")/../build.sh"

out=$root/target/package/linux
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
# ffmpeg beside the program, where the export looks first (nettai_video::mp4::find_ffmpeg); and its notice (GPL:
# what it is built from, where that source is).
if [ -n "${NETTAI_BUNDLE_FFMPEG:-}" ]; then
    cp "$NETTAI_BUNDLE_FFMPEG" "$app/ffmpeg"
else
    fetch_ffmpeg ffmpeg-linux-x86_64
    cp "$fetched" "$app/ffmpeg"
fi
chmod 755 "$app/ffmpeg"
fetch_ffmpeg ffmpeg-NOTICE.txt
cp "$fetched" "$app/ffmpeg-NOTICE.txt"

# The release (Velopack's: docs/app.md §19).
"${VPK:-vpk}" pack --packId nettai --packVersion "$version" --packTitle nettai --packAuthors nettai --packDir "$app" \
    --mainExe nettai --icon "$icons/png/256x256.png" --categories "Game;ActionGame;" --outputDir "$out/releases"

echo "$out/releases"
