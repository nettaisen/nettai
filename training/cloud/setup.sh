#!/usr/bin/env bash
# Sets up a fresh Ubuntu machine for training (§11): build tools, Rust, uv, the Python environment and the
# bridge (nettai_gym, built with maturin), then a smoke test.
#
# Run it on the machine, inside a checkout of the repo, after uploading the packs' timings (timings.sh):
#
#   training/cloud/setup.sh [TIMINGS_DIR]      # default: training/timings
#
# The smoke test runs the trainer's tests, opens a pool on the timings and trains a few updates.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
training="$(dirname "$here")"
timings="$(realpath "${1:-$training/timings}")"

if [ ! -f "$timings/exe6/content.toml" ] && [ ! -f "$timings/exe5/content.toml" ] && [ ! -f "$timings/exe4/content.toml" ]; then
    echo "setup: no pack timings in $timings (make them locally with training/cloud/timings.sh and upload them)" >&2
    exit 1
fi

echo "== system packages"
sudo apt-get update -q
sudo DEBIAN_FRONTEND=noninteractive apt-get install -yq build-essential pkg-config git tmux rsync curl ca-certificates \
    libwayland-dev  # nettai-platform's file dialogs (rfd) need it on Linux, even in the headless bridge

echo "== Rust"
if ! command -v cargo >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/cargo" ]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
# shellcheck disable=SC1091
source "$HOME/.cargo/env"

echo "== uv"
if ! command -v uv >/dev/null 2>&1 && [ ! -x "$HOME/.local/bin/uv" ]; then
    curl -LsSf https://astral.sh/uv/install.sh | sh
fi
export PATH="$HOME/.local/bin:$PATH"

echo "== Python environment"
cd "$training"
uv sync

echo "== the bridge (nettai_gym)"
uv run maturin develop --release --uv --manifest-path bridge/Cargo.toml

echo "== smoke test"
uv run --no-sync pytest -q
NETTAI_PACKS="$timings" uv run --no-sync python - <<'EOF'
import nettai_gym
pool = nettai_gym.Pool(games=["exe6"], envs=2, threads=1, seed=1)
spec = pool.spec()
print("pool ok: spec", spec["hash"][:12], "sets", ", ".join(spec["sets"]))
EOF
smoke="runs/smoke-$(date +%Y%m%d-%H%M%S)"
NETTAI_PACKS="$timings" uv run --no-sync python -m nettai_ai.train --run-dir "$smoke" --config configs/smoke.toml --updates 3
echo "setup: done (smoke run in training/$smoke). Start a run with training/cloud/run.sh."
