"""Checkpoints: the model and optimizer, the run's config, the git commit, the spec, the league and the rest
of the trainer's state, so a run resumes where it stopped and an export knows what it came from."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any

import torch

REPO = Path(__file__).resolve().parents[2]


def git_info() -> dict[str, Any]:
    """The checkout's commit and whether it has uncommitted changes. A run outside a git checkout fails: a
    checkpoint must say which code made it."""
    try:
        commit = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, text=True, check=True
        ).stdout.strip()
        status = subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=no"], cwd=REPO, capture_output=True, text=True, check=True
        ).stdout
    except (OSError, subprocess.CalledProcessError) as e:
        raise RuntimeError(f"checkpoint: {REPO} is not a git checkout (a checkpoint names its commit): {e}") from e
    return {"commit": commit, "dirty": bool(status.strip())}


def checkpoint_dir(run_dir: Path) -> Path:
    return Path(run_dir) / "checkpoints"


def save(run_dir: Path, update: int, state: dict[str, Any]) -> Path:
    d = checkpoint_dir(run_dir)
    d.mkdir(parents=True, exist_ok=True)
    path = d / f"ckpt_{update:07d}.pt"
    tmp = path.with_suffix(".tmp")
    torch.save(state, tmp)
    tmp.replace(path)
    (d / "latest").write_text(path.name + "\n")
    return path


def latest(run_dir: Path) -> Path:
    d = checkpoint_dir(run_dir)
    marker = d / "latest"
    if not marker.exists():
        raise FileNotFoundError(f"no checkpoint in {d}")
    return d / marker.read_text().strip()


def load(path: str | Path, device: str | torch.device = "cpu") -> dict[str, Any]:
    """A checkpoint file, or a run directory (its latest checkpoint)."""
    p = Path(path)
    if p.is_dir():
        p = latest(p)
    return torch.load(p, map_location=device, weights_only=False)
