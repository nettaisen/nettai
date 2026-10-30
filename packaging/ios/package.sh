#!/usr/bin/env bash
# nettai.app for iOS, zipped: target/package/ios/<kind>/nettai-ios-<kind>.zip, for the Simulator on a Mac
# (aarch64-apple-ios-sim) or for a device (aarch64-apple-ios).
#
#   NETTAI_IOS=simulator packaging/ios/package.sh
#
# The bundle: nettai (the program), Info.plist (packaging/ios/Info.plist: full screen, every orientation, the
# extended gamepad profile, the local network's DNS-SD service; its words in the other languages, *.lproj), its
# icons, LICENSE and THIRD-PARTY-NOTICES.txt. No ffmpeg (iOS exports video with VideoToolbox), no content (the app
# downloads it, as every package does), no ROM.
#
# NETTAI_IOS                 simulator (the default) or device
# NETTAI_VERSION             the version, if set; else the app crate's
# IOS_SIGN_IDENTITY          with IOS_PROVISIONING_PROFILE, a device build signed for a device with that identity
# IOS_PROVISIONING_PROFILE   (an Apple Developer account's); else a device build is unsigned, for signing later.
#                            The Simulator's is signed ad hoc, which is all it asks.
set -euo pipefail
# shellcheck source-path=SCRIPTDIR source=../build.sh
source "$(dirname "$0")/../build.sh"

kind=${NETTAI_IOS:-simulator}
case $kind in
    simulator) target=aarch64-apple-ios-sim platform=iPhoneSimulator ;;
    device) target=aarch64-apple-ios platform=iPhoneOS ;;
    *) echo "NETTAI_IOS is simulator or device, not $kind" >&2; exit 1 ;;
esac
# The oldest iOS the app runs on: the framework calls it makes (UTType, UIWindowScene's windows, GameController's
# extended gamepad) and Info.plist's UILaunchScreen are iOS 14's and before; 15 is the floor every device that runs
# 14 can update to.
minimum_os=15.0
export IPHONEOS_DEPLOYMENT_TARGET=$minimum_os

out=$root/target/package/ios/$kind
rm -rf "$out"
mkdir -p "$out"
build_app "$target"

app=$out/nettai.app
mkdir -p "$app"
cp "$program" "$app/nettai"
chmod 755 "$app/nettai"
sed -e "s/@VERSION@/$version/" -e "s/@SHORT_VERSION@/${version%%-*}/" -e "s/@PLATFORM@/$platform/" \
    -e "s/@MINIMUM_OS@/$minimum_os/" "$root/packaging/ios/Info.plist" > "$app/Info.plist"
# Info.plist's words in the other languages (the local network's question).
for strings in "$root"/packaging/ios/*.lproj; do
    cp -R "$strings" "$app/"
done
printf 'APPL????' > "$app/PkgInfo"
cp "$root/LICENSE" "$app/LICENSE"
cp "$notices" "$app/THIRD-PARTY-NOTICES.txt"

# The icons, full bleed (iOS rounds the corners itself), at the sizes Info.plist names: the iPhone's 60 points at
# two and three times, the iPad's 76 and 83.5 at two.
command -v rsvg-convert >/dev/null || { echo "rsvg-convert is needed (librsvg)" >&2; exit 1; }
icon() { # <pixels> <file>
    rsvg-convert --width "$1" --height "$1" --keep-aspect-ratio "$root/docs/logo/nettai-icon.svg" --output "$app/$2"
}
icon 120 AppIcon60x60@2x.png
icon 180 AppIcon60x60@3x.png
icon 152 AppIcon76x76@2x~ipad.png
icon 167 AppIcon83.5x83.5@2x~ipad.png

if [ "$kind" = simulator ]; then
    codesign --force --sign - --timestamp=none "$app"
elif [ -n "${IOS_SIGN_IDENTITY:-}" ] && [ -n "${IOS_PROVISIONING_PROFILE:-}" ]; then
    cp "$IOS_PROVISIONING_PROFILE" "$app/embedded.mobileprovision"
    entitlements=$out/entitlements.plist
    security cms -D -i "$IOS_PROVISIONING_PROFILE" > "$out/profile.plist"
    /usr/libexec/PlistBuddy -x -c 'Print :Entitlements' "$out/profile.plist" > "$entitlements"
    codesign --force --sign "$IOS_SIGN_IDENTITY" --entitlements "$entitlements" "$app"
fi

(cd "$out" && ditto -c -k --keepParent nettai.app "nettai-ios-$kind.zip")
echo "$out/nettai-ios-$kind.zip"
