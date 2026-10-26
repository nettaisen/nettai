"""Publish (§10.5): a checkpoint's network as the weights nettai's dummy that fights downloads.

    uv run python -m nettai_ai.publish runs/NAME/checkpoints/ckpt_0002000.pt [--line 0.11] [--out FILE] [--release]

No weights are built into nettai: the nettai-ai repository (github.com/nettaisen/nettai-ai) publishes networks as
releases, tagged <line>-r<revision>, and its Pages workflow serves each line's newest, which the app downloads
(crates/nettai/src/screens/practice/weights.rs). This exports the checkpoint (`nettai_ai.export`, with its own check
that the file rebuilds the network), keeps of its metadata only what nettai reads to play it ([`PLAYED`]: nothing of
the run it came from), checks the file plays in nettai (the tests that need trained weights, run on it:
`NETTAI_AI_WEIGHTS` names it), and writes it to `--out`. With `--release` it then publishes the file as the line's
next release of the repository (the GitHub CLI, signed in), which rebuilds the site. The line is nettai's major and
minor version, the workspace's by default: weights for a line are played by every nettai of it, so a change to what
the network sees (the observation spec) comes with a new minor version and a network trained for it.
"""

from __future__ import annotations

import argparse
import hashlib
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
REPO = "nettaisen/nettai-ai"

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


def next_revision(line: str) -> int:
    """The line's next release revision: one past its releases' highest (`<line>-r<N>`)."""
    out = subprocess.run(["gh", "release", "list", "-R", REPO, "--limit", "1000", "--json", "tagName", "--jq", ".[].tagName"],
                         capture_output=True, text=True, check=True).stdout
    revisions = [int(m[1]) for tag in out.split() if (m := re.fullmatch(rf"{re.escape(line)}-r(\d+)", tag))]
    return max(revisions, default=0) + 1


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("checkpoint", help="a checkpoint file or a run directory (its latest checkpoint)")
    ap.add_argument("--line", default=None, help="the nettai minor version it is for (default: the workspace's)")
    ap.add_argument("--out", default=FILE, help=f"where the checked file is written (default: {FILE})")
    ap.add_argument("--release", action="store_true", help="publish it as the line's next release of the repository")
    a = ap.parse_args()
    line = a.line or workspace_line()
    if not re.fullmatch(r"\d+\.\d+", line):
        raise SystemExit(f"publish: --line {line!r} isn't a major.minor version")
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
    out = Path(a.out).resolve()
    out.write_bytes(data)
    sha = hashlib.sha256(data).hexdigest()
    print(f"publish: {out} ({len(data):,} bytes), SHA-256 {sha}")
    if not a.release:
        print("publish: not released (--release publishes it)")
        return
    tag = f"{line}-r{next_revision(line)}"
    with tempfile.TemporaryDirectory() as tmp:
        # (The release's file must be named FILE, whatever --out was.)
        asset = Path(tmp) / FILE
        asset.write_bytes(data)
        notes = f"The network nettai {line}'s practice dummy plays.\n\nSHA-256 {sha}"
        subprocess.run(["gh", "release", "create", tag, str(asset), "-R", REPO, "--target", "main", "--title", f"{line} {tag.split('-')[1]}",
                        "--notes", notes], check=True)
    print(f"publish: released {tag}; the repository's Pages workflow puts it on the site")


if __name__ == "__main__":
    torch.set_grad_enabled(False)
    main()
