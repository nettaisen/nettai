#!/usr/bin/env bash
# nettai.app, and Velopack's release of it, on macOS: target/package/macos/releases/ (nettai-osx-Portable.zip, the
# app, which the player puts where they like, /Applications or anywhere, and which updates itself there; no installer
# package: the user, "i don't want to ship a pkg file for mac"; and the update package and its feed the app's updater
# reads from the GitHub release: nettai-<version>-osx-full.nupkg, releases.osx.json, assets.osx.json, RELEASES-osx).
#
#   packaging/macos/package.sh
#
# The bundle: Contents/MacOS/{nettai, ffmpeg}, Contents/Resources/{content/, nettai.icns, LICENSE,
# THIRD-PARTY-NOTICES.txt, ffmpeg-NOTICE.txt}. No ROM and no extracted asset is in it: the app extracts each game's
# pack from the player's own copy.
#
# NETTAI_VERSION         the version, if set; else the app crate's
# NETTAI_UNIVERSAL=1     arm64 and x86_64 in one program (lipo), and in one ffmpeg; else the host's alone
# NETTAI_BUNDLE_FFMPEG   the ffmpeg to bundle (for the architectures built); else the release packaging/ffmpeg.sha256
#                        pins, downloaded and checked
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

out=$root/target/package/macos
rm -rf "$out"
mkdir -p "$out"

if [ "${NETTAI_UNIVERSAL:-}" = 1 ]; then
    arch=universal
    build_app aarch64-apple-darwin
    arm64=$program
    build_app x86_64-apple-darwin
    lipo -create -output "$out/nettai" "$arm64" "$program"
else
    arch=$(uname -m)
    build_app
    cp "$program" "$out/nettai"
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
copy_content "$app/Contents/Resources"
cp "$root/LICENSE" "$app/Contents/Resources/LICENSE"
cp "$notices" "$app/Contents/Resources/THIRD-PARTY-NOTICES.txt"

# ffmpeg beside the program, where the export looks first (nettai_video::mp4::find_ffmpeg), the release's two
# slices joined for a universal app; and its notice (GPL: what it is built from, where that source is).
ffmpeg=$app/Contents/MacOS/ffmpeg
if [ -n "${NETTAI_BUNDLE_FFMPEG:-}" ]; then
    cp "$NETTAI_BUNDLE_FFMPEG" "$ffmpeg"
elif [ "$arch" = universal ]; then
    fetch_ffmpeg ffmpeg-macos-arm64
    arm64=$fetched
    fetch_ffmpeg ffmpeg-macos-x86_64
    lipo -create -output "$ffmpeg" "$arm64" "$fetched"
else
    fetch_ffmpeg "ffmpeg-macos-$arch"
    cp "$fetched" "$ffmpeg"
fi
chmod 755 "$ffmpeg"
fetch_ffmpeg ffmpeg-NOTICE.txt
cp "$fetched" "$app/Contents/Resources/ffmpeg-NOTICE.txt"

# The release (Velopack's: docs/app.md §19). With an identity vpk signs the bundle itself, everything in it with the
# hardened runtime, as notarizing asks; without one it is signed ad hoc here (an Apple silicon Mac runs nothing
# unsigned; Gatekeeper still asks the player to allow it once).
pack=(pack --packId nettai --packVersion "$version" --packTitle nettai --packAuthors nettai --packDir "$app" --mainExe nettai
    --icon "$app/Contents/Resources/nettai.icns" --noInst --outputDir "$out/releases")
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
