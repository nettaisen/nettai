#!/usr/bin/env bash
# The app's program for one Mac architecture, built for a package: target/package/macos/slices/<arch>/ (nettai, and
# licenses.txt, the build's third-party notices). packaging/macos/package.sh packages the slices built here: one for
# an architecture's app, both for the universal app. The release workflow builds the two in jobs of their own, at
# once, and packages them in another.
#
#   NETTAI_MACOS_ARCH=aarch64 packaging/macos/slice.sh
#
# NETTAI_MACOS_ARCH      aarch64 or x86_64; else this Mac's own architecture
set -euo pipefail
# shellcheck source-path=SCRIPTDIR source=../build.sh
source "$(dirname "$0")/../build.sh"

host=$(uname -m)
arch=${NETTAI_MACOS_ARCH:-${host/arm64/aarch64}}
case $arch in
    aarch64 | x86_64) ;;
    *) echo "NETTAI_MACOS_ARCH is aarch64 or x86_64, not $arch" >&2; exit 1 ;;
esac

build_app "$arch-apple-darwin"
out=$root/target/package/macos/slices/$arch
rm -rf "$out"
mkdir -p "$out"
cp "$program" "$out/nettai"
cp "$notices" "$out/licenses.txt"
echo "$out"
