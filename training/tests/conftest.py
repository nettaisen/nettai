from __future__ import annotations

import numpy as np
import pytest
import torch

from nettai_ai.config import ModelConfig
from nettai_ai.spec import spec_hash

TINY_MODEL = dict(d=16, heads=2, ffn=32, token_hidden=16, pool_heads=2, summary=16, hidden=16, head_hidden=16, pointer_hidden=8)


def tiny_spec() -> dict:
    raw = {
        "buckets": 64,
        "fight_actions": 3,
        "screen_actions": 4,  # 2 slots, then 2 keys
        "sets": {
            "global": {"count": 1, "width": 4, "ids": 0},
            "objects": {"count": 3, "width": 3, "ids": 2},
            "slots": {"count": 2, "width": 2, "ids": 2},
            "rules": {"count": 4, "width": 2, "ids": 2},  # pool-only by default
        },
    }
    raw["hash"] = spec_hash(raw)
    return raw


@pytest.fixture(autouse=True)
def _seed():
    torch.manual_seed(0)
    np.random.seed(0)


@pytest.fixture
def tiny_cfg() -> ModelConfig:
    return ModelConfig(**TINY_MODEL)
