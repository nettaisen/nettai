#!/usr/bin/env bash
# nettai's APK, for Android on arm64 (Android 8 and later): target/package/android/nettai-<version>.apk.
#
#   packaging/android/package.sh
#
# The native library (the app's crate's library, built as a cdylib for Android alone: its android_main is what the
# GameActivity runs) built by cargo for aarch64-linux-android with the NDK's compilers, stripped, with the NDK's libc++_shared.so; the build's notices and
# the license as assets; the launcher icon; the certificate verifier's component (rustls-platform-verifier's AAR,
# pinned in verifier.sha256); then Gradle (the wrapper here: Gradle 9.8.1, its zip checked by its SHA-256; AGP 9.5.0,
# build.gradle.kts) compiles the crates' Kotlin, adds GameActivity and AppCompat, shrinks the code (R8), packages it
# all with the library stored uncompressed and aligned for 16 KB pages, and signs it (unless it is to be unsigned).
#
# ANDROID_HOME                the SDK, with platforms;android-36 and build-tools
# ANDROID_NDK_HOME            the NDK, whose clang compiles the library's C (Luau, ring, GameActivity's) and links it
# JAVA_HOME                   a JDK 17 or later, which Gradle and AGP run on
# NETTAI_ANDROID_KEYSTORE     a keystore to sign with, its password in NETTAI_ANDROID_KEYSTORE_PASSWORD and the key's
#                             alias in NETTAI_ANDROID_KEY_ALIAS; else a debug key made in target/package/android (an
#                             APK signed with one key doesn't install over one signed with another)
# NETTAI_ANDROID_UNSIGNED     1: the APK isn't signed at all, for whoever installs it to sign (the release's; Android
#                             installs no APK unsigned)
# NETTAI_ANDROID_DEBUGGABLE   1: the APK says it is debuggable (`adb shell run-as` reads and writes its files)
# NETTAI_ICONS                a folder packaging/icons.sh made; else it is made here (needs librsvg and ImageMagick)
# NETTAI_VERSION              the version, if set; else the workspace's
set -euo pipefail
# shellcheck source-path=SCRIPTDIR source=../build.sh
source "$(dirname "$0")/../build.sh"

sdk=${ANDROID_HOME:?set ANDROID_HOME to the Android SDK}
ndk=${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to the Android NDK}
: "${JAVA_HOME:?set JAVA_HOME to a JDK 17 or later}"
[ -f "$sdk/platforms/android-36/android.jar" ] || { echo "no platforms;android-36 in $sdk: sdkmanager 'platforms;android-36'" >&2; exit 1; }
# Android 8 (the oldest whose AAudio cpal plays through): the C's API level, as build.gradle.kts's minSdk.
min_sdk=26
case $(uname -s) in
    Darwin) llvm=$ndk/toolchains/llvm/prebuilt/darwin-x86_64/bin ;;
    *) llvm=$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin ;;
esac

out=$root/target/package/android
rm -rf "$out/jniLibs" "$out/assets" "$out/res"
mkdir -p "$out/jniLibs/arm64-v8a" "$out/assets" "$out/res/mipmap-xxxhdpi"

# The native library, release (`NETTAI_PACKAGED`: the app plays the content it downloads), with the NDK's clang for
# the C it compiles and links, stripped of its debug information; and the build's third-party notices, as
# packaging/build.sh's `build_app` finds them.
export CC_aarch64_linux_android=$llvm/aarch64-linux-android$min_sdk-clang
export CXX_aarch64_linux_android=$llvm/aarch64-linux-android$min_sdk-clang++
export AR_aarch64_linux_android=$llvm/llvm-ar
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$llvm/aarch64-linux-android$min_sdk-clang
messages=$(NETTAI_PACKAGED=1 cargo rustc --release --locked -p nettai --lib --crate-type cdylib \
    --target aarch64-linux-android --message-format=json-render-diagnostics --manifest-path "$root/Cargo.toml")
library=$(grep -o '"filenames":\["[^"]*libnettai\.so"' <<<"$messages" | sed 's/.*\["//; s/"$//' | tail -n 1)
[ -f "$library" ] || { echo "the build named no libnettai.so" >&2; exit 1; }
notices=
while IFS= read -r dir; do
    [ -f "$dir/licenses.txt" ] && notices=$dir/licenses.txt
done < <(sed -n 's/.*"reason":"build-script-executed".*"out_dir":"\([^"]*\)".*/\1/p' <<<"$messages")
[ -n "$notices" ] || { echo "the build named no licenses.txt (crates/nettai/build.rs)" >&2; exit 1; }
"$llvm/llvm-strip" --strip-debug -o "$out/jniLibs/arm64-v8a/libnettai.so" "$library"
# (The C++ the library compiles (Luau's) links the NDK's libc++ as a library of its own, which the APK carries beside
# it: Android's are the system's alone.)
if "$llvm/llvm-readelf" -d "$library" | grep -q 'NEEDED.*\[libc++_shared\.so\]'; then
    "$llvm/llvm-strip" --strip-unneeded -o "$out/jniLibs/arm64-v8a/libc++_shared.so" \
        "$llvm/../sysroot/usr/lib/aarch64-linux-android/libc++_shared.so"
fi
cp "$notices" "$out/assets/THIRD-PARTY-NOTICES.txt"
cp "$root/LICENSE" "$out/assets/LICENSE"

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

# The launcher icon (Android scales it to the screen's density).
icons=${NETTAI_ICONS:-}
if [ -z "$icons" ]; then
    icons=$out/icons
    "$root/packaging/icons.sh" "$icons"
fi
cp "$icons/png/256x256.png" "$out/res/mipmap-xxxhdpi/ic_launcher.png"

# The version code is the version's numbers (0.17.0: 17000).
IFS=. read -r major minor patch <<<"${version%%-*}"
code=$((major * 1000000 + minor * 1000 + patch))

# Unsigned, or the keystore given, else a debug key made once.
if [ -n "${NETTAI_ANDROID_UNSIGNED:-}" ]; then
    signing=() built=nettai-release-unsigned.apk
else
    if [ -n "${NETTAI_ANDROID_KEYSTORE:-}" ]; then
        keystore=$NETTAI_ANDROID_KEYSTORE
        password=${NETTAI_ANDROID_KEYSTORE_PASSWORD:?set NETTAI_ANDROID_KEYSTORE_PASSWORD}
        alias=${NETTAI_ANDROID_KEY_ALIAS:?set NETTAI_ANDROID_KEY_ALIAS}
    else
        keystore=$out/debug.keystore password=android alias=nettai-debug
        [ -f "$keystore" ] || "$JAVA_HOME/bin/keytool" -genkeypair -keystore "$keystore" -storepass "$password" \
            -alias "$alias" -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=nettai debug" >/dev/null
    fi
    signing=(-Pnettai.keystore="$keystore" -Pnettai.keystorePassword="$password" -Pnettai.keyAlias="$alias")
    built=nettai-release.apk
fi

# Gradle: its project cache and Kotlin's folder (sessions, error logs) with the rest of the build's, not in the
# repository. Its last APK goes first: AGP otherwise writes the new one into it in place, which leaves the old
# library's room behind and doubles the APK.
rm -f "$out/gradle/outputs/apk/release/$built"
"$root/packaging/android/gradlew" --project-dir "$root/packaging/android" --project-cache-dir "$out/gradle-cache" \
    --quiet assembleRelease -Pkotlin.project.persistent.dir="$out/kotlin" \
    -Pnettai.out="$out" -Pnettai.version="$version" -Pnettai.versionCode="$code" -Pnettai.verifier="$out/$aar" \
    ${signing[@]+"${signing[@]}"} -Pnettai.debuggable="${NETTAI_ANDROID_DEBUGGABLE:-}"
apk=$out/nettai-$version.apk
cp "$out/gradle/outputs/apk/release/$built" "$apk"
echo "$apk"
