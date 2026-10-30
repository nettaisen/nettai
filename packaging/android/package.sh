#!/usr/bin/env bash
# nettai's APK, for Android on arm64 (Android 8 and later): target/package/android/nettai-<version>.apk.
#
#   packaging/android/package.sh
#
# No Gradle: the native library (crates/nettai-android-app, whose android_main the NativeActivity runs) built by cargo
# for aarch64-linux-android with the NDK's compilers; the app's Kotlin (each crate's android/**/*.kt: nettai-dialogs'
# NettaiFiles) compiled by kotlinc against the platform's android.jar, and dexed by d8 with the Kotlin standard
# library and the certificate verifier's component (rustls-platform-verifier's AAR, pinned in verifier.sha256: its
# Kotlin wants the standard library too, which is in the APK once); the manifest (AndroidManifest.xml) and the
# resources (the launcher icon, the verifier's network security config) linked by aapt2; the native library stored
# uncompressed and the whole aligned for 16 KB pages (zipalign); then signed (apksigner).
#
# ANDROID_HOME                the SDK, with build-tools (aapt2, d8, zipalign, apksigner) and platforms;android-36
# ANDROID_NDK_HOME            the NDK, whose clang compiles the library's C (Luau, ring) and links it
# KOTLINC                     kotlinc, else the one on the PATH; its home's lib/kotlin-stdlib.jar goes into the APK
# KOTLIN_HOME                 Kotlin's home, where kotlinc's isn't found
# NETTAI_ANDROID_KEYSTORE     a keystore to sign with, its password in NETTAI_ANDROID_KEYSTORE_PASSWORD and the key's
#                             alias in NETTAI_ANDROID_KEY_ALIAS; else a debug key made in target/package/android (an
#                             APK signed with one key doesn't install over one signed with another)
# NETTAI_ANDROID_DEBUGGABLE   1: the APK says it is debuggable (`adb shell run-as` reads and writes its files)
# NETTAI_ICONS                a folder packaging/icons.sh made; else it is made here (needs librsvg and ImageMagick)
# NETTAI_VERSION              the version, if set; else the workspace's
set -euo pipefail
# shellcheck source-path=SCRIPTDIR source=../build.sh
source "$(dirname "$0")/../build.sh"

sdk=${ANDROID_HOME:?set ANDROID_HOME to the Android SDK}
ndk=${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to the Android NDK}
# Android 8 (the oldest whose AAudio cpal plays through), and the platform the code is compiled against.
min_sdk=26
platform=$sdk/platforms/android-36/android.jar
[ -f "$platform" ] || { echo "no $platform: sdkmanager 'platforms;android-36'" >&2; exit 1; }
tools=$(find "$sdk/build-tools" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -n 1)
[ -x "$tools/aapt2" ] || { echo "no build-tools in $sdk: sdkmanager 'build-tools;36.0.0'" >&2; exit 1; }
case $(uname -s) in
    Darwin) llvm=$ndk/toolchains/llvm/prebuilt/darwin-x86_64/bin ;;
    *) llvm=$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin ;;
esac
kotlinc=$(command -v "${KOTLINC:-kotlinc}") || { echo "kotlinc is needed (Homebrew's kotlin)" >&2; exit 1; }
# (Kotlin's home: $KOTLIN_HOME, else kotlinc's, as Kotlin's own distribution lays it out (bin/kotlinc, lib/) or
# Homebrew's (bin/kotlinc, libexec/lib/).)
home=$(dirname "$(dirname "$(realpath "$kotlinc")")")
stdlib=
for dir in ${KOTLIN_HOME:+"$KOTLIN_HOME"} "$home" "$home/libexec"; do
    [ -f "$dir/lib/kotlin-stdlib.jar" ] && { stdlib=$dir/lib/kotlin-stdlib.jar; break; }
done
[ -n "$stdlib" ] || { echo "no Kotlin standard library in $home (set KOTLIN_HOME)" >&2; exit 1; }

out=$root/target/package/android
work=$out/work
rm -rf "$work"
mkdir -p "$work/apk/lib/arm64-v8a" "$work/classes" "$work/res/mipmap-xxxhdpi" "$work/res/xml" "$work/verifier"

# The native library, release (`NETTAI_PACKAGED`: the app plays the content it downloads), with the NDK's clang for
# the C it compiles and links, stripped of its debug information; and the build's third-party notices, as
# packaging/build.sh's `build_app` finds them.
export CC_aarch64_linux_android=$llvm/aarch64-linux-android$min_sdk-clang
export CXX_aarch64_linux_android=$llvm/aarch64-linux-android$min_sdk-clang++
export AR_aarch64_linux_android=$llvm/llvm-ar
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$llvm/aarch64-linux-android$min_sdk-clang
messages=$(NETTAI_PACKAGED=1 cargo build --release --locked -p nettai-android-app --target aarch64-linux-android \
    --message-format=json-render-diagnostics --manifest-path "$root/Cargo.toml")
library=$(grep -o '"filenames":\["[^"]*libnettai_app\.so"' <<<"$messages" | sed 's/.*\["//; s/"$//' | tail -n 1)
[ -f "$library" ] || { echo "the build named no libnettai_app.so" >&2; exit 1; }
notices=
while IFS= read -r dir; do
    [ -f "$dir/licenses.txt" ] && notices=$dir/licenses.txt
done < <(sed -n 's/.*"reason":"build-script-executed".*"out_dir":"\([^"]*\)".*/\1/p' <<<"$messages")
[ -n "$notices" ] || { echo "the build named no licenses.txt (crates/nettai/build.rs)" >&2; exit 1; }
"$llvm/llvm-strip" --strip-debug -o "$work/apk/lib/arm64-v8a/libnettai_app.so" "$library"
# (The C++ the library compiles (Luau's) links the NDK's libc++ as a library of its own, which the APK carries beside
# it: Android's are the system's alone.)
if "$llvm/llvm-readelf" -d "$library" | grep -q 'NEEDED.*\[libc++_shared\.so\]'; then
    "$llvm/llvm-strip" --strip-unneeded -o "$work/apk/lib/arm64-v8a/libc++_shared.so" \
        "$llvm/../sysroot/usr/lib/aarch64-linux-android/libc++_shared.so"
fi
mkdir -p "$work/apk/assets"
cp "$notices" "$work/apk/assets/THIRD-PARTY-NOTICES.txt"
cp "$root/LICENSE" "$work/apk/assets/LICENSE"

# The certificate verifier's component, of the version Cargo.lock has, checked against its pin.
verifier=$(awk '$0 == "name = \"rustls-platform-verifier-android\"" { getline; gsub(/version = |"/, ""); print }' "$root/Cargo.lock")
aar=rustls-platform-verifier-$verifier.aar
sum=$(awk -v file="$aar" '$2 == file { print $1 }' "$root/packaging/android/verifier.sha256")
[ -n "$sum" ] || { echo "packaging/android/verifier.sha256 pins no $aar (Cargo.lock has $verifier)" >&2; exit 1; }
if [ "$(sha256_of "$out/$aar")" != "$sum" ]; then
    curl --fail --location --silent --show-error --output "$out/$aar" \
        "https://raw.githubusercontent.com/rustls/rustls-platform-verifier/maven-archive/android-release-support/maven/org/rustls/rustls-platform-verifier/$verifier/$aar"
    [ "$(sha256_of "$out/$aar")" = "$sum" ] || { echo "$aar isn't the one pinned" >&2; rm -f "$out/$aar"; exit 1; }
fi
unzip -q -o "$out/$aar" classes.jar res/xml/network_security_config.xml -d "$work/verifier"
cp "$work/verifier/res/xml/network_security_config.xml" "$work/res/xml/"

# The app's Kotlin, and the dex of it with the verifier's classes and the Kotlin standard library.
kotlin=()
while IFS= read -r file; do kotlin+=("$file"); done < <(find "$root/crates" -path '*/android/*' -name '*.kt' | sort)
"$kotlinc" -nowarn -jvm-target 17 -cp "$platform" -d "$work/classes" "${kotlin[@]}"
classes=()
while IFS= read -r file; do classes+=("$file"); done < <(find "$work/classes" -name '*.class' | sort)
"$tools/d8" --release --min-api "$min_sdk" --lib "$platform" --output "$work/apk" \
    "${classes[@]}" "$work/verifier/classes.jar" "$stdlib"

# The launcher icon (Android scales it to the screen's density).
icons=${NETTAI_ICONS:-}
if [ -z "$icons" ]; then
    icons=$out/icons
    "$root/packaging/icons.sh" "$icons"
fi
cp "$icons/png/256x256.png" "$work/res/mipmap-xxxhdpi/ic_launcher.png"

# The manifest and the resources, linked: the version code is the version's numbers (0.17.0: 17000).
IFS=. read -r major minor patch <<<"${version%%-*}"
code=$((major * 1000000 + minor * 1000 + patch))
sed -e "s/@VERSION@/$version/" -e "s/@VERSION_CODE@/$code/" "$root/packaging/android/AndroidManifest.xml" > "$work/AndroidManifest.xml"
"$tools/aapt2" compile --dir "$work/res" -o "$work/res.zip"
link=(link -I "$platform" --manifest "$work/AndroidManifest.xml" -R "$work/res.zip" --min-sdk-version "$min_sdk"
    --target-sdk-version 34 -o "$work/linked.apk")
[ "${NETTAI_ANDROID_DEBUGGABLE:-}" = 1 ] && link+=(--debug-mode)
"$tools/aapt2" "${link[@]}"

# The dex and the assets compressed, the native library stored as it is (Android maps it from the APK); aligned.
(cd "$work/apk" && zip -q -r "$work/linked.apk" classes.dex assets && zip -q -0 -r "$work/linked.apk" lib)
"$tools/zipalign" -f -P 16 4 "$work/linked.apk" "$work/aligned.apk"

# Signed: with the keystore given, else with a debug key made once.
if [ -n "${NETTAI_ANDROID_KEYSTORE:-}" ]; then
    keystore=$NETTAI_ANDROID_KEYSTORE
    password=${NETTAI_ANDROID_KEYSTORE_PASSWORD:?set NETTAI_ANDROID_KEYSTORE_PASSWORD}
    alias=${NETTAI_ANDROID_KEY_ALIAS:?set NETTAI_ANDROID_KEY_ALIAS}
else
    keystore=$out/debug.keystore password=android alias=nettai-debug
    [ -f "$keystore" ] || keytool -genkeypair -keystore "$keystore" -storepass "$password" -alias "$alias" \
        -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=nettai debug" >/dev/null
fi
apk=$out/nettai-$version.apk
"$tools/apksigner" sign --ks "$keystore" --ks-pass "pass:$password" --ks-key-alias "$alias" --out "$apk" "$work/aligned.apk"
rm -f "$apk.idsig"
echo "$apk"
