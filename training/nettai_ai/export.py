"""Export (§8): a checkpoint's network as safetensors for the Rust forward pass, and a parity fixture.

    uv run python -m nettai_ai.export runs/NAME [--out nettai-ai.safetensors] [--fixture nettai-ai.fixture.safetensors]

The weights file holds every tensor of the model's state dict under its PyTorch name (MODEL.md lists them)
and, in its header's metadata (all strings):

    format          "nettai-ai"
    format_version  "1"
    spec_hash       the observation spec's hash; the Rust side refuses weights whose hash isn't its encoder's
    architecture    JSON: the widths, the spec's sets in order, activation, LayerNorm eps
    levels          JSON: §10's difficulty knobs (delay ticks, temperature, epsilon, greedy) by level
    k               ticks a fight decision is held in training (the env's k)
    screen_decisions  screen decisions before only OK is allowed (the env's limit)
    git_commit, git_dirty, update, decisions    provenance

The fixture holds a run of one seat's recorded decisions (from training, so real observations) and what the
network computes for them, step by step from h0: inputs `x.<set>`, `ids.<set>`, `mask.<set>`, `legal_fight`,
`legal_screen`, `phase`, `reset` (zero the GRU state before that decision), `previous` (the previous decision's
action as the GRU reads it, `model.previous_action`), `h0`; outputs `fight_raw`,
`screen_raw` (absent slots -inf), `fight` and `screen` (legal masks applied, -inf), `value`, `hidden` (the GRU
state after each decision). The Rust port must reproduce the outputs.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
from pathlib import Path
from typing import Any

import torch
from safetensors import safe_open
from safetensors.torch import save_file

from . import checkpoint
from .config import LEVELS, config_from_dict
from .model import FORMAT, FORMAT_VERSION, Model, mask_logits
from .spec import parse_spec


def model_from_checkpoint(state: dict[str, Any]) -> Model:
    cfg = config_from_dict(state["config"])
    model = Model(parse_spec(state["spec"]), cfg.model)
    model.load_state_dict(state["model"])
    model.eval()
    return model


def metadata(model: Model, state: dict[str, Any]) -> dict[str, str]:
    cfg = config_from_dict(state["config"])
    return {
        "format": FORMAT,
        "format_version": str(FORMAT_VERSION),
        "spec_hash": model.spec.hash,
        "architecture": json.dumps(model.architecture(), sort_keys=True),
        "levels": json.dumps({k: dataclasses.asdict(v) for k, v in LEVELS.items()}, sort_keys=True),
        # How the env asked for decisions in training, which the app's player asks the same way.
        "k": str(cfg.env.k),
        "screen_decisions": str(cfg.env.screen_decisions),
        "git_commit": state["git"]["commit"],
        "git_dirty": str(bool(state["git"]["dirty"])).lower(),
        "update": str(state["update"]),
        "decisions": str(state["decisions"]),
    }


def save_weights(model: Model, path: str | Path, meta: dict[str, str]) -> None:
    tensors = {k: v.detach().cpu().contiguous() for k, v in model.state_dict().items()}
    save_file(tensors, str(path), metadata=meta)


def load_weights(path: str | Path) -> tuple[Model, dict[str, str]]:
    """The model a weights file describes, rebuilt from its architecture and loaded strictly."""
    with safe_open(str(path), framework="pt") as f:
        meta = f.metadata()
        if meta.get("format") != FORMAT or meta.get("format_version") != str(FORMAT_VERSION):
            raise ValueError(f"{path}: not a {FORMAT} v{FORMAT_VERSION} weights file")
        tensors = {k: f.get_tensor(k) for k in f.keys()}
    model = Model.from_architecture(meta["architecture"])
    if model.spec.hash != meta["spec_hash"]:
        raise ValueError(f"{path}: the architecture's spec hash isn't the file's spec_hash")
    model.load_state_dict(tensors, strict=True)
    model.eval()
    return model, meta


@torch.no_grad()
def run_fixture(model: Model, fx: dict[str, Any]) -> dict[str, torch.Tensor]:
    """The network's outputs on a recorded sequence, decision by decision, as the Rust side runs it."""
    n = fx["reset"].shape[0]
    h = fx["h0"].unsqueeze(0).clone()
    out: dict[str, list[torch.Tensor]] = {k: [] for k in ("fight_raw", "screen_raw", "fight", "screen", "value", "hidden")}
    for i in range(n):
        obs = {k: {name: v[i : i + 1] for name, v in fx[k].items()} for k in ("x", "ids", "mask")}
        if bool(fx["reset"][i]):
            h = torch.zeros_like(h)
        fight, screen, value, h = model.step(obs, h, fx["previous"][i : i + 1].long())
        out["fight_raw"].append(fight[0])
        out["screen_raw"].append(screen[0])
        out["fight"].append(mask_logits(fight, fx["legal_fight"][i : i + 1])[0])
        out["screen"].append(mask_logits(screen, fx["legal_screen"][i : i + 1])[0])
        out["value"].append(value[0])
        out["hidden"].append(h[0])
    return {k: torch.stack(v) for k, v in out.items()}


def save_fixture(model: Model, fx: dict[str, Any], path: str | Path, meta: dict[str, str]) -> dict[str, torch.Tensor]:
    outputs = run_fixture(model, fx)
    tensors: dict[str, torch.Tensor] = {}
    for k in ("x", "ids", "mask"):
        for name, v in fx[k].items():
            tensors[f"{k}.{name}"] = v
    for k in ("legal_fight", "legal_screen", "phase", "reset", "previous"):
        tensors[k] = fx[k]
    tensors["h0"] = fx["h0"]
    tensors.update(outputs)
    save_file({k: v.detach().cpu().contiguous() for k, v in tensors.items()}, str(path), metadata=meta)
    return outputs


def export(ckpt: str | Path, out: str | Path, fixture: str | Path) -> None:
    state = checkpoint.load(ckpt)
    model = model_from_checkpoint(state)
    if state.get("fixture") is None:
        raise SystemExit(f"{ckpt}: the checkpoint has no recorded decisions for the parity fixture")
    meta = metadata(model, state)
    save_weights(model, out, meta)
    expected = save_fixture(model, state["fixture"], fixture, {k: meta[k] for k in ("format", "format_version", "spec_hash")})
    # The file must rebuild the same network: run the fixture again on what was written.
    again, _ = load_weights(out)
    got = run_fixture(again, state["fixture"])
    for k, v in expected.items():
        if not torch.equal(v, got[k]):
            raise SystemExit(f"export: {out} doesn't reproduce {k} (max diff {float((v - got[k]).abs().nan_to_num().max())})")
    print(f"wrote {out} ({sum(t.numel() for t in model.state_dict().values()):,} values, spec {model.spec.hash[:12]})")
    print(f"wrote {fixture} ({state['fixture']['reset'].shape[0]} decisions)")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("checkpoint", help="a checkpoint file or a run directory (its latest checkpoint)")
    ap.add_argument("--out", default="nettai-ai.safetensors")
    ap.add_argument("--fixture", default=None, help="default: OUT with .fixture.safetensors")
    a = ap.parse_args()
    out = Path(a.out)
    fixture = Path(a.fixture) if a.fixture else out.with_name(out.stem + ".fixture.safetensors")
    export(a.checkpoint, out, fixture)


if __name__ == "__main__":
    main()
