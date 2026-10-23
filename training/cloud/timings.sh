#!/usr/bin/env bash
# Copies ONLY the packs' timing data out of data/exe* for the cloud machine (§11): each pack's content.toml
# and assets.toml, and each sprite's animations.json and sprite.json. No picture or sound leaves.
#
#   training/cloud/timings.sh [PACKS_DIR] [OUT_DIR]     # defaults: $NETTAI_PACKS or data/, training/timings
#
# Then upload OUT_DIR to the machine (rsync -a training/timings/ HOST:REPO/training/timings/).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
training="$(dirname "$here")"
repo="$(dirname "$training")"
packs="${1:-${NETTAI_PACKS:-$repo/data}}"
out="${2:-$training/timings}"

shopt -s nullglob
found=0
rm -rf "$out"
for pack in "$packs"/exe*; do
    [ -f "$pack/content.toml" ] || continue
    name="$(basename "$pack")"
    found=1
    mkdir -p "$out/$name"
    cp "$pack/content.toml" "$out/$name/"
    [ -f "$pack/assets.toml" ] && cp "$pack/assets.toml" "$out/$name/"
    for sprite in "$pack"/graphics/sprites/*/; do
        s="$(basename "$sprite")"
        for f in animations.json sprite.json; do
            if [ -f "$sprite$f" ]; then
                mkdir -p "$out/$name/graphics/sprites/$s"
                cp "$sprite$f" "$out/$name/graphics/sprites/$s/"
            fi
        done
    done
done
if [ "$found" = 0 ]; then
    echo "timings: no packs (exe*/content.toml) in $packs" >&2
    exit 1
fi

# Only the four kinds of file may be in the folder.
stray="$(find "$out" -type f ! -name content.toml ! -name assets.toml ! -name animations.json ! -name sprite.json)"
if [ -n "$stray" ]; then
    echo "timings: unexpected files in $out:" >&2
    echo "$stray" >&2
    exit 1
fi
echo "timings: $(find "$out" -type f | wc -l | tr -d ' ') files, $(du -sh "$out" | cut -f1) in $out"
