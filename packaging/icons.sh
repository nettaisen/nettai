#!/usr/bin/env bash
# The app's icons, made from docs/logo/nettai-icon.svg (nothing raster is committed):
#
#   <out>/png/<n>x<n>.png   full bleed, 16 to 512 pixels (Linux's hicolor sizes)
#   <out>/nettai.ico        full bleed, 16 to 256 pixels (the Windows program and its installer)
#   <out>/nettai.iconset/   macOS's set for iconutil: the rounded square on Apple's grid (824 of 1024, centered,
#                           a transparent margin around it), so that it is no larger than other apps in the Dock
#
# Needs rsvg-convert (librsvg) and ImageMagick (`magick`, or version 6's `convert`).
#
#   packaging/icons.sh <out>
set -euo pipefail

out=${1:?usage: packaging/icons.sh <out-dir>}
root=$(cd "$(dirname "$0")/.." && pwd)
svg=$root/docs/logo/nettai-icon.svg
if command -v magick >/dev/null; then im=(magick); else im=(convert); fi
command -v rsvg-convert >/dev/null || { echo "rsvg-convert is needed (librsvg)" >&2; exit 1; }
command -v "${im[0]}" >/dev/null || { echo "ImageMagick is needed" >&2; exit 1; }

mkdir -p "$out/png" "$out/nettai.iconset"

render() { # <size> <file>
    rsvg-convert --width "$1" --height "$1" --keep-aspect-ratio "$svg" --output "$2"
}

for size in 16 24 32 48 64 128 256 512; do
    render "$size" "$out/png/${size}x${size}.png"
done

ico_files=()
for size in 16 24 32 48 64 128 256; do ico_files+=("$out/png/${size}x${size}.png"); done
"${im[@]}" "${ico_files[@]}" "$out/nettai.ico"

# macOS: each size of the set, the artwork at 824/1024 of it, centered on a transparent canvas.
for size in 16 32 128 256 512; do
    for scale in 1 2; do
        px=$((size * scale))
        art=$(( (px * 824 + 512) / 1024 ))
        name=icon_${size}x${size}
        [ "$scale" = 2 ] && name=${name}@2x
        render "$art" "$out/nettai.iconset/art.png"
        "${im[@]}" "$out/nettai.iconset/art.png" -background none -gravity center -extent "${px}x${px}" "PNG32:$out/nettai.iconset/$name.png"
    done
done
rm "$out/nettai.iconset/art.png"

echo "icons written to $out"
