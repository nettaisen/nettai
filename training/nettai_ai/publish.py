"""Publish (§10.5): a checkpoint's network as the weights nettai's dummy that fights downloads.

    uv run python -m nettai_ai.publish runs/NAME/checkpoints/ckpt_0002000.pt --repo ../../nettai-ai [--line 0.11] [--push]

No weights are built into nettai: the nettai-ai repository (github.com/nettaisen/nettai-ai) keeps one network for
each of nettai's minor versions, and the app downloads its own line's (crates/nettai/src/screens/practice/weights.rs).
This exports the checkpoint (`nettai_ai.export`, with its own check that the file rebuilds the network), keeps of
its metadata only what nettai reads to play it ([`PLAYED`]: nothing of the run it came from), checks the file plays
in nettai (the tests that need trained weights, run on it: `NETTAI_AI_WEIGHTS` names it), then writes it to the
repository checkout `--repo` as `<line>/nettai-ai.safetensors` and its SHA-256 in `index.json` (by line), and
commits there; `--push` pushes too. The line is nettai's major and minor version, the workspace's by default: weights for a line
are played by every nettai of it, so a change to what the network sees (the observation spec) comes with a new
minor version and a network trained for it.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path

import torch
from safetensors import safe_open
from safetensors.torch import load_file, save_file

from .export import export

ROOT = Path(__file__).resolve().parents[2]  # the nettai repository
FILE = "nettai-ai.safetensors"

# The metadata nettai reads to play a network (net.rs, `Net::from_safetensors`): the rest of the export's (the
# commit, the update and decisions it is from) isn't published.
PLAYED = ("format", "format_version", "spec_hash", "architecture", "levels", "k", "screen_decisions")

# The tests that need trained weights, each a cargo command run from the repository's root.
CHECKS = [
    ["cargo", "test", "-p", "nettai-training", "--features", "live", "--lib", "ai::tests::weights_to_publish", "--", "--ignored"],
    ["cargo", "test", "-p", "nettai", "--features", "ai", "--lib", "the_dummy_that_fights", "--", "--ignored"],
]


def workspace_line() -> str:
    """nettai's major and minor version, from the workspace's Cargo.toml."""
    text = (ROOT / "Cargo.toml").read_text()
    m = re.search(r'^\[workspace\.package\][^\[]*?^version\s*=\s*"(\d+)\.(\d+)\.', text, re.M | re.S)
    if not m:
        raise SystemExit("publish: no [workspace.package] version in Cargo.toml")
    return f"{m[1]}.{m[2]}"


def check(weights: Path) -> None:
    for cmd in CHECKS:
        print("check:", " ".join(cmd))
        r = subprocess.run(cmd, cwd=ROOT, env={**os.environ, "NETTAI_AI_WEIGHTS": str(weights)})
        if r.returncode != 0:
            raise SystemExit(f"publish: the check failed: {' '.join(cmd)}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("checkpoint", help="a checkpoint file or a run directory (its latest checkpoint)")
    ap.add_argument("--repo", required=True, help="a checkout of the nettai-ai repository")
    ap.add_argument("--line", default=None, help="the nettai minor version it is for (default: the workspace's)")
    ap.add_argument("--push", action="store_true", help="push the commit too")
    a = ap.parse_args()
    line = a.line or workspace_line()
    if not re.fullmatch(r"\d+\.\d+", line):
        raise SystemExit(f"publish: --line {line!r} isn't a major.minor version")
    repo = Path(a.repo).resolve()
    if not (repo / ".git").exists():
        raise SystemExit(f"publish: {repo} isn't a checkout of the nettai-ai repository")
    ckpt = Path(a.checkpoint).resolve()
    with tempfile.TemporaryDirectory() as tmp:
        exported = Path(tmp) / "exported.safetensors"
        export(str(ckpt), exported, Path(tmp) / "fixture.safetensors")
        with safe_open(str(exported), "pt") as f:
            meta = f.metadata()
        out = Path(tmp) / FILE
        save_file(load_file(str(exported)), str(out), metadata={k: meta[k] for k in PLAYED})
        check(out)
        data = out.read_bytes()
    (repo / line).mkdir(exist_ok=True)
    (repo / line / FILE).write_bytes(data)
    index_path = repo / "index.json"
    index = json.loads(index_path.read_text()) if index_path.exists() else {}
    index[line] = hashlib.sha256(data).hexdigest()
    index_path.write_text(json.dumps(index, indent=2, sort_keys=True) + "\n")
    print(f"publish: {repo / line / FILE} ({len(data):,} bytes), index.json's {line}")
    subprocess.run(["git", "add", f"{line}/{FILE}", "index.json"], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-q", "-m", f"nettai {line}: new weights"], cwd=repo, check=True)
    if a.push:
        subprocess.run(["git", "push", "-q"], cwd=repo, check=True)
        print("publish: pushed")
    else:
        print(f"publish: committed in {repo}; push it to publish (git -C {repo} push)")


if __name__ == "__main__":
    torch.set_grad_enabled(False)
    main()
