#!/usr/bin/env bash
# An AppImage of the app, on x86_64 Linux: target/package/linux/nettai-<version>-linux-x86_64.AppImage.
#
#   packaging/linux/package.sh
#
# The AppDir: AppRun, nettai.desktop, nettai.png, usr/lib/nettai/{nettai, content/} (the content beside the
# program, where the app looks for it), the icons in usr/share/icons/hicolor, and LICENSE and
# THIRD-PARTY-NOTICES.txt in usr/share/doc/nettai. No ROM and no extracted asset is in it: the app extracts each
# game's pack from the player's own copy. The system's libraries (glibc, ALSA, fontconfig, OpenGL, X11 or
# Wayland) are the system's: build it on an old distribution (the workflow's is Ubuntu 22.04) to reach more.
#
# NETTAI_VERSION   the version (the workflow's, from the tag); else the app crate's
# NETTAI_ICONS     a folder packaging/icons.sh made; else it is made here (needs librsvg and ImageMagick)
#
# appimagetool and the AppImage runtime are downloaded at pinned versions and checked against their digests.
set -euo pipefail
# shellcheck source-path=SCRIPTDIR source=../build.sh
source "$(dirname "$0")/../build.sh"

appimagetool_url=https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
appimagetool_sha256=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
runtime_url=https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64
runtime_sha256=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d

out=$root/target/package/linux
tools=$root/target/package/tools
rm -rf "$out"
mkdir -p "$out" "$tools"

fetch() { # <url> <sha256> <file>
    if ! echo "$2  $3" | sha256sum --check --status 2>/dev/null; then
        curl --fail --location --silent --show-error --output "$3" "$1"
        echo "$2  $3" | sha256sum --check
    fi
}
fetch "$appimagetool_url" "$appimagetool_sha256" "$tools/appimagetool"
fetch "$runtime_url" "$runtime_sha256" "$tools/runtime-x86_64"
chmod +x "$tools/appimagetool"

build_app ""

icons=${NETTAI_ICONS:-}
if [ -z "$icons" ]; then
    icons=$out/icons
    "$root/packaging/icons.sh" "$icons"
fi

appdir=$out/nettai.AppDir
lib=$appdir/usr/lib/nettai
mkdir -p "$lib" "$appdir/usr/share/applications" "$appdir/usr/share/doc/nettai"
# (The release profile keeps line tables for the log's backtraces; the program's symbols stay, its debug info goes.)
cp "$program" "$lib/nettai"
strip --strip-debug "$lib/nettai"
copy_content "$lib"
cp "$root/LICENSE" "$appdir/usr/share/doc/nettai/LICENSE"
cp "$notices" "$appdir/usr/share/doc/nettai/THIRD-PARTY-NOTICES.txt"

cp "$root/packaging/linux/AppRun" "$appdir/AppRun"
chmod +x "$appdir/AppRun"
cp "$root/packaging/linux/nettai.desktop" "$appdir/nettai.desktop"
cp "$root/packaging/linux/nettai.desktop" "$appdir/usr/share/applications/nettai.desktop"
for png in "$icons"/png/*.png; do
    size=$(basename "$png" .png)
    mkdir -p "$appdir/usr/share/icons/hicolor/$size/apps"
    cp "$png" "$appdir/usr/share/icons/hicolor/$size/apps/nettai.png"
done
cp "$icons/png/256x256.png" "$appdir/nettai.png"
ln -s nettai.png "$appdir/.DirIcon"

appimage=$out/nettai-$version-linux-x86_64.AppImage
# (Extracted and run, so that no FUSE is needed where it is built.)
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$tools/appimagetool" --no-appstream --runtime-file "$tools/runtime-x86_64" "$appdir" "$appimage"

echo "$appimage"
