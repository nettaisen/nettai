#!/usr/bin/env bash
# Starts a training run in tmux on the cloud machine (§11), with TensorBoard beside it on localhost:6006.
#
#   training/cloud/run.sh NAME [train.py arguments...]
#
# e.g. training/cloud/run.sh first --config configs/cloud.toml
#      training/cloud/run.sh first --resume            (continue the run's latest checkpoint)
#      training/cloud/run.sh second --config configs/cloud.toml --init-from runs/first
#                                                      (a new run from the first run's latest network)
#
# Watch it with `tmux attach -t train-NAME`; see TensorBoard from your machine through an ssh tunnel:
#   ssh -L 6006:localhost:6006 HOST     then open http://localhost:6006
set -euo pipefail

if [ $# -lt 1 ]; then
    echo "usage: $0 NAME [train.py arguments...]" >&2
    exit 2
fi
name="$1"
shift
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
training="$(dirname "$here")"
timings="${NETTAI_PACKS:-$training/timings}"
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

run="runs/$name"
mkdir -p "$training/$run"
args=$(printf ' %q' "$@")
session="train-$name"
if tmux has-session -t "$session" 2>/dev/null; then
    echo "run: tmux session $session is already running" >&2
    exit 1
fi
tmux new-session -d -s "$session" -c "$training" \
    "NETTAI_PACKS=$(printf %q "$timings") uv run --no-sync python -m nettai_ai.train --run-dir $run$args 2>&1 | tee -a $run/train.log"
if ! tmux has-session -t tensorboard 2>/dev/null; then
    tmux new-session -d -s tensorboard -c "$training" "uv run --no-sync tensorboard --logdir runs --host 127.0.0.1 --port 6006"
fi
echo "run: started $session (log: training/$run/train.log); TensorBoard in tmux session tensorboard"
