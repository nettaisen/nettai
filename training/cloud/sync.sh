#!/usr/bin/env bash
# Copies runs back from the cloud machine (§11): checkpoints, snapshots, TensorBoard logs and train.log.
# Run it on your machine:
#
#   training/cloud/sync.sh HOST [REMOTE_REPO] [RUN]     # REMOTE_REPO default: nettai (in the remote home)
#
# Without RUN it copies every run. Re-running copies only what changed.
set -euo pipefail

if [ $# -lt 1 ]; then
    echo "usage: $0 HOST [REMOTE_REPO] [RUN]" >&2
    exit 2
fi
host="$1"
remote="${2:-nettai}"
run="${3:-}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
training="$(dirname "$here")"

src="$remote/training/runs/${run:+$run/}"
dst="$training/runs/${run:+$run/}"
mkdir -p "$dst"
rsync -az "$host:$src" "$dst"
echo "sync: $host:$src → $dst"
