#!/usr/bin/env bash
# nettai.app, and Velopack's release of it, on macOS, for one architecture: target/package/macos/<arch>/releases/
# (nettai-<channel>-Portable.zip, the app, which the player puts where they like, /Applications or anywhere, and
# which updates itself there; no installer package: the user, "i don't want to ship a pkg file for mac"; and the
# update package and its feed the app's updater reads from the GitHub release: nettai-<version>-<channel>-full.nupkg,
# releases.<channel>.json, assets.<channel>.json, RELEASES-<channel>).
#
#   NETTAI_MACOS_ARCH=aarch64 packaging/macos/package.sh
#
# Each architecture is its own package on its own Velopack channel, named as Velopack names the runtime (`vpk pack
# --runtime`, which the universal app has none of), so that each one's files and feed sit beside the others' in one
# release:
#
#   aarch64     Apple silicon                              osx-arm64
#   x86_64      Intel                                      osx-x64
#   universal   both, the two programs joined by lipo      osx (a Mac's only package up to 0.12.0: the installs
#                                                               from it update through it; docs/app.md §19)
#
# The bundle: Contents/MacOS/{nettai, ffmpeg}, Contents/Resources/{nettai.icns, LICENSE, THIRD-PARTY-NOTICES.txt,
# ffmpeg-NOTICE.txt}. No content, no ROM and no extracted asset is in it: the app downloads each game's content
# (docs/design/content-deployment.md §5) and extracts each game's pack from the player's own copy.
#
# NETTAI_MACOS_ARCH      aarch64, x86_64 or universal (above); else this Mac's own architecture
# NETTAI_VERSION         the version, if set; else the app crate's
# NETTAI_BUNDLE_FFMPEG   the ffmpeg to bundle (for the architecture packaged); else the release
#                        packaging/ffmpeg.sha256 pins, downloaded and checked
# NETTAI_ICONS           a folder packaging/icons.sh made; else it is made here (needs librsvg and ImageMagick)
# MACOS_SIGN_IDENTITY    a Developer ID Application identity in the keychain to sign the app with (vpk signs it);
#                        else it is signed ad hoc here
# MACOS_NOTARY_PROFILE   with the identity, a notarytool keychain profile (`xcrun notarytool store-credentials`):
#                        the app is notarized and stapled; else it isn't
# MACOS_KEYCHAIN         the keychain those are in; else the default
# VPK                    Velopack's vpk (the version crates/nettai's velopack is: 1.2.161); else vpk on the PATH
set -euo pipefail
# shellcheck source-path=SCRIPTDIR source=../build.sh
source "$(dirname "$0")/../build.sh"

host=$(uname -m)
arch=${NETTAI_MACOS_ARCH:-${host/arm64/aarch64}}
# The slices the program is made of (Rust's target names), the channel, and Velopack's runtime.
case $arch in
    aarch64) slices=(aarch64) channel=osx-arm64 runtime=osx-arm64 ;;
    x86_64) slices=(x86_64) channel=osx-x64 runtime=osx-x64 ;;
    universal) slices=(aarch64 x86_64) channel=osx runtime= ;;
    *) echo "NETTAI_MACOS_ARCH is aarch64, x86_64 or universal, not $arch" >&2; exit 1 ;;
esac

# One program of its slices': one alone as it is, two joined by lipo.
join_slices() { # <output> <program>...
    local output=$1
    shift
    if [ $# = 1 ]; then cp "$1" "$output"; else lipo -create -output "$output" "$@"; fi
}

out=$root/target/package/macos/$arch
rm -rf "$out"
mkdir -p "$out"

# Each slice built for its target, whichever package it is for: one packaged already, as the release workflow
# packages each architecture before the universal app, is cargo's to reuse, and nothing is compiled twice.
programs=()
for slice in "${slices[@]}"; do
    build_app "$slice-apple-darwin"
    programs+=("$program")
done
join_slices "$out/nettai" "${programs[@]}"

icons=${NETTAI_ICONS:-}
if [ -z "$icons" ]; then
    icons=$out/icons
    "$root/packaging/icons.sh" "$icons"
fi

app=$out/nettai.app
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
mv "$out/nettai" "$app/Contents/MacOS/nettai"
# (CFBundleShortVersionString is numbers alone: a tag's "-beta.1" is left out of it.)
sed -e "s/@VERSION@/$version/" -e "s/@SHORT_VERSION@/${version%%-*}/" "$root/packaging/macos/Info.plist" > "$app/Contents/Info.plist"
printf 'APPL????' > "$app/Contents/PkgInfo"
iconutil -c icns -o "$app/Contents/Resources/nettai.icns" "$icons/nettai.iconset"
cp "$root/LICENSE" "$app/Contents/Resources/LICENSE"
cp "$notices" "$app/Contents/Resources/THIRD-PARTY-NOTICES.txt"

# ffmpeg beside the program, where the export looks first (nettai_video::mp4::find_ffmpeg): the release's program
# for each slice (its arm64 is Rust's aarch64), joined as the app's are; and its notice (GPL: what it is built from,
# where that source is).
ffmpeg=$app/Contents/MacOS/ffmpeg
if [ -n "${NETTAI_BUNDLE_FFMPEG:-}" ]; then
    cp "$NETTAI_BUNDLE_FFMPEG" "$ffmpeg"
else
    ffmpegs=()
    for slice in "${slices[@]}"; do
        fetch_ffmpeg "ffmpeg-macos-${slice/aarch64/arm64}"
        ffmpegs+=("$fetched")
    done
    join_slices "$ffmpeg" "${ffmpegs[@]}"
fi
chmod 755 "$ffmpeg"
fetch_ffmpeg ffmpeg-NOTICE.txt
cp "$fetched" "$app/Contents/Resources/ffmpeg-NOTICE.txt"

# The release (Velopack's: docs/app.md §19). With an identity vpk signs the bundle itself, everything in it with the
# hardened runtime, as notarizing asks; without one it is signed ad hoc here (an Apple silicon Mac runs nothing
# unsigned; Gatekeeper still asks the player to allow it once).
pack=(pack --packId nettai --packVersion "$version" --packTitle nettai --packAuthors nettai --packDir "$app" --mainExe nettai
    --channel "$channel" --icon "$app/Contents/Resources/nettai.icns" --noInst --outputDir "$out/releases")
[ -n "$runtime" ] && pack+=(--runtime "$runtime")
if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
    pack+=(--signAppIdentity "$MACOS_SIGN_IDENTITY")
    [ -n "${MACOS_NOTARY_PROFILE:-}" ] && pack+=(--notaryProfile "$MACOS_NOTARY_PROFILE")
    [ -n "${MACOS_KEYCHAIN:-}" ] && pack+=(--keychain "$MACOS_KEYCHAIN")
else
    codesign --force --sign - "$ffmpeg"
    codesign --force --sign - "$app"
fi
"${VPK:-vpk}" "${pack[@]}"

echo "$out/releases"
