#!/usr/bin/env bash
# nettai.app, and Velopack's release of it, on macOS, for one architecture: target/package/macos/<arch>/releases/
# (nettai-<channel>-Portable.zip, the app, which the player puts where they like, /Applications or anywhere, and
# which updates itself there; no installer package: the user, "i don't want to ship a pkg file for mac"; and the
# update package and its feed the app's updater reads from the GitHub release: nettai-<version>-<channel>-full.nupkg,
# releases.<channel>.json, assets.<channel>.json, RELEASES-<channel>).
#
#   NETTAI_MACOS_ARCH=aarch64 packaging/macos/slice.sh       # the program, built (each slice the app is made of)
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
# This compiles nothing: it packages the slices packaging/macos/slice.sh built (target/package/macos/slices/), the
# universal app both of them.
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
# VPK                    Velopack's vpk (the version crates/nettai's velopack is: 1.3.39-g8d2844f); else vpk on the PATH
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

# The program: its slices', as slice.sh built them. The third-party notices are the slices' builds' (each build's
# are its target's crates): one slice's, or for the universal app both, as one where they are the same.
programs=()
notes=()
for slice in "${slices[@]}"; do
    built=$root/target/package/macos/slices/$slice
    [ -f "$built/nettai" ] || { echo "no $slice slice: run NETTAI_MACOS_ARCH=$slice packaging/macos/slice.sh first" >&2; exit 1; }
    programs+=("$built/nettai")
    notes+=("$built/licenses.txt")
done
join_slices "$out/nettai" "${programs[@]}"
# (A slice carried from another job as a workflow's artifact has lost its mode.)
chmod 755 "$out/nettai"
notices=$out/THIRD-PARTY-NOTICES.txt
if [ ${#notes[@]} = 1 ] || cmp -s "${notes[0]}" "${notes[1]}"; then
    cp "${notes[0]}" "$notices"
else
    { cat "${notes[0]}"; printf '\n'; cat "${notes[1]}"; } > "$notices"
fi

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
