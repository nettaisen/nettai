#!/usr/bin/env bash
# nettai.app and a .dmg of it, on macOS: target/package/macos/nettai-<version>-macos-<arch>.dmg.
#
#   packaging/macos/package.sh
#
# The bundle: Contents/MacOS/nettai, Contents/Resources/{content/, nettai.icns, LICENSE, THIRD-PARTY-NOTICES.txt}.
# No ROM and no extracted asset is in it: the app extracts each game's pack from the player's own copy.
#
# NETTAI_VERSION         the version (the workflow's, from the tag); else the app crate's
# NETTAI_UNIVERSAL=1     arm64 and x86_64 in one program (lipo); else the host's alone
# NETTAI_ICONS           a folder packaging/icons.sh made; else it is made here (needs librsvg and ImageMagick)
# MACOS_SIGN_IDENTITY    a Developer ID Application identity in the keychain to sign with; else signed ad hoc
# APPLE_ID, APPLE_TEAM_ID, APPLE_APP_PASSWORD
#                        with an identity, the .dmg is notarized and stapled; else it isn't
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

if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
    codesign --force --options runtime --timestamp --sign "$MACOS_SIGN_IDENTITY" "$app"
else
    # (Ad hoc: an Apple silicon Mac runs nothing unsigned. Gatekeeper still asks the player to allow it once.)
    codesign --force --sign - "$app"
fi

dmg=$out/nettai-$version-macos-$arch.dmg
stage=$out/dmg
mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
hdiutil create -volname "nettai $version" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$dmg"
rm -rf "$stage"

if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
    codesign --force --timestamp --sign "$MACOS_SIGN_IDENTITY" "$dmg"
    if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ] && [ -n "${APPLE_APP_PASSWORD:-}" ]; then
        xcrun notarytool submit "$dmg" --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" --password "$APPLE_APP_PASSWORD" --wait
        xcrun stapler staple "$dmg"
    fi
fi

echo "$dmg"
