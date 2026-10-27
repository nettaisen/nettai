"""Run configuration: nested dataclasses, read from a TOML file, with `--set section.key=value` overrides.

Unknown keys are errors, so a typo in a config file never silently runs the default.
"""

from __future__ import annotations

import dataclasses
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


@dataclass
class ModelConfig:
    """The network's widths (§7). MODEL.md describes the computation these size."""

    # The default fits §7's budget: 2.2 M multiply-adds a decision at the typical fight position (95 tokens, 38
    # of them in the transformer), 5.8 M with every token present.
    d: int = 64  # token width
    heads: int = 4  # attention heads in each transformer layer
    layers: int = 1  # transformer layers over the present tokens of all but the pool-only sets
    ffn: int = 128  # transformer feed-forward hidden width (2 × d)
    token_hidden: int = 32  # each set's token MLP: width → token_hidden → d
    pool_heads: int = 4  # attention-pool heads, each a learned score over tokens
    summary: int = 128  # the pooled summary, the GRU's input
    hidden: int = 256  # GRU state
    head_hidden: int = 128  # fight, key and value heads' hidden width
    pointer_hidden: int = 64  # the screen pointer's hidden width
    pointer_set: str = "slots"  # the token set the screen pointer scores (its count = screen slots)
    # Sets the pool reads but the transformer doesn't: many small tokens whose relations to the field matter
    # little (the rules' scalars: 43 to 79 present every decision), kept out of the attention's cost.
    pool_only: list[str] = field(default_factory=lambda: ["rules"])
    norm_clip: float = 5.0  # normalized inputs are clipped to ±norm_clip


# Named sizes for `--size`. "small" is the default and fits §7's budget; "large" is for trying in the cloud.
SIZES: dict[str, dict[str, Any]] = {
    "small": {},
    "large": {
        "d": 128,
        "heads": 4,
        "layers": 2,
        "ffn": 256,
        "token_hidden": 128,
        "pool_heads": 4,
        "summary": 256,
        "hidden": 256,
        "head_hidden": 256,
        "pointer_hidden": 128,
    },
}


@dataclass
class EnvConfig:
    """`nettai_gym.Pool`'s arguments (§6.6)."""

    games: list[str] = field(default_factory=lambda: ["exe6", "exe5", "exe4"])
    envs: int = 512
    threads: int = 0  # 0: every core the machine has
    seed: int = 1
    content: str | None = None
    packs: str | None = None
    k: int = 4
    # Screen decisions before only OK is allowed: the env's own limit (24), which the bridge takes no argument
    # for; kept here so the weights record what the network was trained with.
    screen_decisions: int = 24
    delay: list[int] = field(default_factory=lambda: [6, 30])
    max_fight_ticks: int = 18000
    # Held-out chips (§9): this fraction of each game's chips never appears in training builds and is favored in
    # evaluation builds (the pool's chip_holdout). 0 turns it off.
    chip_holdout: float = 0.0
    chip_holdout_seed: int = 7
    # The chance a randomized build's lists (its NaviCust, patch cards...) are filled full rather than to a random
    # length (§6.2, `builds::FULL_SHARE`).
    full_lists: float = 0.5
    # Threads that only make builds ahead (the pool's `builders`); 0: the stepping workers make them as they wait,
    # which leaves a step short-handed while a full build (up to 130 ms) is being made.
    builders: int = 4
    mirror: bool = False  # both sides play side 0's build (evaluation's --mirror: play alone decides)
    fixed_builds: int = 0  # each game's builds drawn from this many, made once (0: random every round): a specialist's narrow matchups
    cache: str | None = None  # a directory the pool keeps the chips' fingerprints in (EXE5's take 14 s to make)


@dataclass
class PPOConfig:
    steps: int = 128  # pool steps per rollout
    chunk: int = 128  # decisions per truncated-BPTT chunk (each chunk starts from its stored GRU state)
    epochs: int = 3
    # Chunks per minibatch: 64 (8,192 decision slots) gives the GPU more per call, 1.7× the update's speed of 16
    # on the cloud's RTX 3090 (§11).
    minibatch_chunks: int = 64
    # Each minibatch's passes, its gradients added before the one step: the same step in parts, for a network whose
    # minibatch doesn't fit the GPU's memory in one pass (the extra-large run's on 32 GB).
    accumulate: int = 1
    lr: float = 3e-4
    lr_final: float = 0.1  # the learning rate falls linearly to lr × lr_final over run.updates
    adam_eps: float = 1e-5
    gamma: float = 0.997
    lam: float = 0.95
    clip: float = 0.2
    value_clip: float = 0.2
    entropy: float = 0.01
    value_coef: float = 0.5
    max_grad_norm: float = 0.5
    name_dropout: float = 0.2  # per token, the learner plays without its name ids (rollouts and updates only)
    norm_warmup_steps: int = 32  # pool steps that set the input normalization before the first update


@dataclass
class RewardConfig:
    """§6.4: the round's result, and the HP difference weighted down over training.

    The pool's hp_gain is already a fraction of the round's max HP, so a KO's shaping is the same at any HP of
    the curriculum. A stall costs (the user, 2026-10-24: "you should penalize winning by stalling more because
    that's boring"): a seat that didn't lose a round decided by the judge's ruling gives up `judged_cost` (a win
    there worth result − judged_cost, a draw −judged_cost; a loss is a loss), and a round that reaches the
    fight's limit is worth `limit` to each seat."""

    result: float = 1.0
    judged_cost: float = 0.75
    limit: float = -1.0
    hp_start: float = 1.0
    hp_end: float = 0.2
    hp_anneal_updates: int = 3000


@dataclass
class CurriculumConfig:
    """Both sides' max HP per round starts low and rises to the full 1000 as the learner gets better.

    Rounds draw their HP from (max(start, level - band), level). The level rises by `step` when the learner's
    win rate against `bot` reaches `winrate` (over at least `min_rounds` rounds at this level), or after
    `max_updates` updates at one level, whichever is first. The last `final_fraction` of run.updates is played
    at `final` only. Uses the pool's `hp=(lo, hi)` argument and `set_hp(lo, hi)` (from the next round).
    """

    start: int = 250
    final: int = 1000
    step: int = 125
    band: int = 250
    bot: str = "masher"
    winrate: float = 0.75
    min_rounds: int = 100
    max_updates: int = 500
    final_fraction: float = 0.25


@dataclass
class LeagueConfig:
    """§8's mix. The snapshot share goes to self-play until the first snapshot exists."""

    self_play: float = 0.50
    snapshots: float = 0.35
    bots: float = 0.15
    # Rounds against another run's networks (`external_dir`'s weight files, rescanned each rollout): an exploiter
    # plays the main run's latest so (external 1); the main run plays the exploiter's snapshots so (§8).
    external: float = 0.0
    external_dir: str | None = None
    bot_names: list[str] = field(default_factory=lambda: ["still", "mover", "masher", "shooter", "dodger"])
    snapshot_every: int = 10  # updates
    pfsp_power: float = 2.0  # a snapshot's weight is (1 - learner's win rate against it) ** power
    # A round keeps its opponent to its end (rounds last about eight rollouts), and each snapshot in play costs
    # the collector a network pass every step (about 1.1 s a rollout on the cloud machine), so the number in
    # play is bounded: new rounds draw from `active` snapshots, and at most `draining` dropped ones still play
    # out their rounds. One active slot turns over a rollout while fewer than `draining` are draining.
    active: int = 2  # snapshots new snapshot rounds draw from, by PFSP weight
    draining: int = 2  # dropped from the active set, still in rounds begun before
    cache: int = 8  # snapshot networks kept loaded on the device (active + draining, twice with an external share)
    # Snapshots kept on disk: past this, the one the learner beats most is deleted (its record stays, its
    # PFSP weight is near zero anyway). Neither an active nor an in-play one.
    keep: int = 200
    elo_k: float = 16.0
    winrate_window: int = 200  # rounds the learner's win rate against a snapshot is averaged over


@dataclass
class EvalConfig:
    """Periodic evaluation during training against the scripted players, on `games` (held-out games go here)."""

    games: list[str] = field(default_factory=list)
    bots: list[str] = field(default_factory=lambda: ["still", "mover", "masher", "shooter", "dodger"])
    every: int = 0  # updates; 0 = never
    envs: int = 40
    threads: int = 4
    rounds: int = 2  # counted rounds per env (after the first, which is discarded)
    level: str = "normal"
    hp: int = 1000  # evaluation plays at full HP


@dataclass
class RunConfig:
    updates: int = 20000
    checkpoint_every: int = 50
    device: str = "auto"  # auto: cuda, else cpu
    # torch's CPU threads, per thread (they run at once, beside the pool's own): the collector's (gathering each
    # step's tokens; on a CPU device the rollout's inference too; 0 leaves torch's default) and the learner's
    # (on a CPU device the update; 0: as the collector's).
    torch_threads: int = 0
    update_threads: int = 0
    seed: int = 1
    fixture_decisions: int = 16  # decisions of one seat kept in each checkpoint for export's parity fixture


@dataclass
class Config:
    model: ModelConfig = field(default_factory=ModelConfig)
    env: EnvConfig = field(default_factory=EnvConfig)
    ppo: PPOConfig = field(default_factory=PPOConfig)
    reward: RewardConfig = field(default_factory=RewardConfig)
    curriculum: CurriculumConfig = field(default_factory=CurriculumConfig)
    league: LeagueConfig = field(default_factory=LeagueConfig)
    eval: EvalConfig = field(default_factory=EvalConfig)
    run: RunConfig = field(default_factory=RunConfig)


@dataclass(frozen=True)
class Level:
    """§10's difficulty knobs, as the Python side plays them (evaluate) and as export writes them for Rust."""

    name: str
    delay: int  # reaction delay in ticks (pinned with pool.set_delay)
    temperature: float  # logits are divided by this before sampling
    epsilon: float  # chance of a uniformly random legal action
    greedy: bool  # take the best legal action instead of sampling


# Reaction times from §10 at 60 ticks a second: 400 ms → 24, 270 ms → 16, 170 ms → 10.
LEVELS: dict[str, Level] = {
    "easy": Level("easy", delay=24, temperature=1.5, epsilon=0.1, greedy=False),
    "normal": Level("normal", delay=16, temperature=1.0, epsilon=0.0, greedy=False),
    "hard": Level("hard", delay=10, temperature=1.0, epsilon=0.0, greedy=True),
}


def _from_dict(cls, data: dict[str, Any], path: str):
    if not isinstance(data, dict):
        raise ValueError(f"config {path or 'root'}: expected a table, got {data!r}")
    kwargs = {}
    fields = {f.name: f for f in dataclasses.fields(cls)}
    for key, value in data.items():
        if key not in fields:
            raise ValueError(f"config: unknown key {path + key!r} (known: {', '.join(fields)})")
        f = fields[key]
        sub = f.default_factory() if f.default_factory is not dataclasses.MISSING else None
        if dataclasses.is_dataclass(sub):
            kwargs[key] = _from_dict(type(sub), value, f"{path}{key}.")
        else:
            kwargs[key] = value
    return cls(**kwargs)


def config_from_dict(data: dict[str, Any]) -> Config:
    return _from_dict(Config, data, "")


def config_to_dict(cfg: Config) -> dict[str, Any]:
    return dataclasses.asdict(cfg)


def _set(data: dict[str, Any], dotted: str, value: Any) -> None:
    keys = dotted.split(".")
    node = data
    for k in keys[:-1]:
        node = node.setdefault(k, {})
        if not isinstance(node, dict):
            raise ValueError(f"override {dotted!r}: {k!r} is not a table")
    node[keys[-1]] = value


def parse_override(text: str) -> tuple[str, Any]:
    """`ppo.lr=1e-4` → ("ppo.lr", 0.0001). The value is read as TOML, else as a bare string."""
    if "=" not in text:
        raise ValueError(f"override {text!r}: expected key=value")
    key, raw = text.split("=", 1)
    try:
        value = tomllib.loads(f"v = {raw}")["v"]
    except tomllib.TOMLDecodeError:
        value = raw
    return key.strip(), value


def load_config(
    path: str | Path | None = None,
    overrides: list[str] = (),
    size: str | None = None,
    base: dict[str, Any] | None = None,
) -> Config:
    """Defaults ← `base` (a checkpoint's config) ← the TOML file ← `size` ← `--set` overrides."""
    data: dict[str, Any] = {}
    if base is not None:
        data = _deep_merge(data, base)
    if path is not None:
        with open(path, "rb") as f:
            data = _deep_merge(data, tomllib.load(f))
    if size is not None:
        if size not in SIZES:
            raise ValueError(f"unknown size {size!r} (known: {', '.join(SIZES)})")
        data = _deep_merge(data, {"model": dict(SIZES[size])})
    for text in overrides:
        key, value = parse_override(text)
        _set(data, key, value)
    return config_from_dict(data)


def _deep_merge(a: dict[str, Any], b: dict[str, Any]) -> dict[str, Any]:
    out = dict(a)
    for k, v in b.items():
        if isinstance(v, dict) and isinstance(out.get(k), dict):
            out[k] = _deep_merge(out[k], v)
        else:
            out[k] = v
    return out
