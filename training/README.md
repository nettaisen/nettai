# training/

The learned opponent's trainer (docs/design/training-ai.md §7–§11): recurrent PPO by self-play in a league,
written in PyTorch, never built with the app. The app runs the exported network with a Rust forward pass
(MODEL.md describes it exactly).

```
nettai_ai/fake.py        FakePool: §6.6's pool in numpy, with a learnable toy game (tests, --fake)
nettai_ai/model.py       the network, its MAC count (python -m nettai_ai.model [--size large])
nettai_ai/rollout.py     per-seat decision streams, compact rollout storage, the collector
nettai_ai/ppo.py         GAE, the PPO update, input normalization
nettai_ai/league.py      self-play, PFSP snapshots, scripted players, Elo
nettai_ai/curriculum.py  the HP curriculum
nettai_ai/device.py      the device, each thread's CUDA stream, copies that don't wait
nettai_ai/train.py       the training loop (collector and learner threads, overlapped) and its CLI
nettai_ai/evaluate.py    checkpoints against bots and each other, per game; Elo
nettai_ai/export.py      safetensors for the Rust side, and a parity fixture
nettai_ai/bench.py       the simulation, inference and update timed apart; update variants; the loop (--loop)
bridge/                  the pyo3 module nettai_gym (its own Cargo workspace)
cloud/                   setup, timings upload, run and sync scripts for the cloud machine
configs/                 smoke.toml (a short run), mac.toml (20 minutes on a Mac), cloud.toml (a full run)
```

## Locally

Everything goes through [uv](https://docs.astral.sh/uv/); nothing is installed globally.

```sh
cd training
uv sync                                   # Python 3.13, torch, numpy, safetensors, tensorboard, pytest
uv run pytest -q                          # about two minutes on a laptop's CPU
uv run python -m nettai_ai.model          # MACs a decision and parameters of the default size
```

On FakePool (no Rust needed):

```sh
uv run python -m nettai_ai.train --run-dir runs/try --config configs/smoke.toml --fake
uv run python -m nettai_ai.evaluate runs/try --fake --envs 20 --rounds 2
uv run python -m nettai_ai.export runs/try --out runs/try/nettai-ai.safetensors
uv run tensorboard --logdir runs
```

On the real engine, build the bridge into this environment once (and after engine changes), then drop
`--fake`. `uv sync` removes it again (it isn't in the lock file), so run with `uv run --no-sync` after building:

```sh
uv run maturin develop --release --uv --manifest-path bridge/Cargo.toml
NETTAI_PACKS=../data uv run --no-sync python -m nettai_ai.train --run-dir runs/mac --config configs/mac.toml
NETTAI_PACKS=../data uv run --no-sync python -m nettai_ai.bench --profile     # where the time goes
NETTAI_PACKS=../data uv run --no-sync python -m nettai_ai.bench --variants base --loop 8   # and the loop's
```

Training overlaps collection with the update: a collector thread plays the next rollout with a copy of the
learner's weights (one update behind) while the learner updates on the last one (docs/design/training-ai.md
§8). There is no other way it runs.

Training writes `RUN_DIR/tb` (TensorBoard), `RUN_DIR/checkpoints` (each with the config, the git commit and the
observation spec; `--resume` continues the latest) and `RUN_DIR/snapshots` (the league's frozen versions).
Settings: a TOML file (`--config`), a named size (`--size large`), and overrides (`--set ppo.lr=1e-4`).
`nettai_ai/config.py` has every setting and its default.

Held-out tests (§9): `--train-games exe5,exe6 --eval-games exe4` trains on two games and evaluates on the third
every `eval.every` updates; `--set env.chip_holdout=0.1` keeps a tenth of each game's chips out of training
builds and favors them in evaluation builds.

## In the cloud

1. Locally, copy the packs' timing files (no pictures or sound) and upload them with the repo:
   ```sh
   training/cloud/timings.sh                      # → training/timings/
   ssh HOST git clone git@github.com:bigfarts/nettai.git nettai     # or rsync the checkout
   rsync -a training/timings/ HOST:nettai/training/timings/
   ```
2. On the machine: `nettai/training/cloud/setup.sh` (Rust, uv, the bridge, tests and a smoke run).
3. Start a run: `nettai/training/cloud/run.sh first --config configs/cloud.toml` (tmux session `train-first`,
   TensorBoard on the machine's localhost:6006; `ssh -L 6006:localhost:6006 HOST` to see it).
4. Locally, fetch checkpoints and logs: `training/cloud/sync.sh HOST nettai first`.
5. Export the best checkpoint (by evaluate's Elo) for the app:
   `uv run python -m nettai_ai.export runs/first --out ../crates/nettai-training/model/nettai-ai.safetensors`.
