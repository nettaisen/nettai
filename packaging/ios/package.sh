#!/usr/bin/env bash
# nettai.app for iOS: for the Simulator on a Mac (aarch64-apple-ios-sim), zipped,
# target/package/ios/simulator/nettai-ios-simulator.zip; or for a device (aarch64-apple-ios), an IPA,
# target/package/ios/device/nettai-ios.ipa (the bundle in Payload/, zipped), which a sideloading tool (AltStore, SideStore,
# Sideloadly) signs with the player's own Apple ID as it installs it.
#
#   NETTAI_IOS=simulator packaging/ios/package.sh
#
# The bundle: nettai (the program), Info.plist (packaging/ios/Info.plist: full screen, every orientation, the
# extended gamepad profile, the local network's DNS-SD service; its words in the other languages, *.lproj), its
# icons, LICENSE and THIRD-PARTY-NOTICES.txt. No ffmpeg (iOS exports video with VideoToolbox), no content (the app
# downloads it, as every package does), no ROM.
#
# NETTAI_IOS                 simulator (the default) or device
# NETTAI_ICONS               a folder packaging/icons.sh made; else it is made here (needs librsvg and ImageMagick)
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
icons=${NETTAI_ICONS:-}
if [ -z "$icons" ]; then
    icons=$out/icons
    "$root/packaging/icons.sh" "$icons"
fi
cp "$icons/png/120x120.png" "$app/AppIcon60x60@2x.png"
cp "$icons/png/180x180.png" "$app/AppIcon60x60@3x.png"
cp "$icons/png/152x152.png" "$app/AppIcon76x76@2x~ipad.png"
cp "$icons/png/167x167.png" "$app/AppIcon83.5x83.5@2x~ipad.png"

if [ "$kind" = simulator ]; then
    codesign --force --sign - --timestamp=none "$app"
elif [ -n "${IOS_SIGN_IDENTITY:-}" ] && [ -n "${IOS_PROVISIONING_PROFILE:-}" ]; then
    cp "$IOS_PROVISIONING_PROFILE" "$app/embedded.mobileprovision"
    entitlements=$out/entitlements.plist
    security cms -D -i "$IOS_PROVISIONING_PROFILE" > "$out/profile.plist"
    /usr/libexec/PlistBuddy -x -c 'Print :Entitlements' "$out/profile.plist" > "$entitlements"
    codesign --force --sign "$IOS_SIGN_IDENTITY" --entitlements "$entitlements" "$app"
fi

if [ "$kind" = simulator ]; then
    (cd "$out" && ditto -c -k --keepParent nettai.app nettai-ios-simulator.zip)
    echo "$out/nettai-ios-simulator.zip"
else
    mkdir "$out/Payload"
    mv "$app" "$out/Payload/"
    (cd "$out" && zip -qrX nettai-ios.ipa Payload)
    echo "$out/nettai-ios.ipa"
fi
