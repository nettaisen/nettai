# Training AI: a learned opponent

The practice dummy's "Fights" choice, played by a neural network trained by self-play. Asked for on 2026-10-23
("i want you to build a state of the art training ai for this game"), after the search-based fighter
(nettai-training, removed in cc24c9a58) was judged "really bad": erratic, wasting chips, not dodging, "it doesn't
really understand the game".

## 0. Decisions

The user's answers, 2026-10-23:

- **Python trains, Rust plays.** Training is PyTorch in `training/` (a uv project). The app runs the trained
  network with a small forward pass written in Rust, on the desktop and in the browser, with no new runtime
  dependency. No Python in the app.
- **It knows what's on screen.** It sees what a player on its side would see: not the opponent's hand, folder order
  or custom-screen picks, not the RNG, not the player's buttons. It reacts after a reaction delay, as a person does.
- **Training runs on a rented cloud machine.** The Mac runs only short smoke tests.
- **"It should be generic enough that it works on any game."** Nothing in it names a game or a game's feature (the
  line in [rust-and-luau.md](rust-and-luau.md) holds for it too). What it sees comes from the engine's shared
  state and the content's own descriptions, so a game added later is played by the same code and the same network.
- **No imitation of replays** ("the replays i have aren't very representative"). It learns from scratch, by self-play.
- **Kept from before:** the AI stays Rust (no Luau AI). Only the player opens the custom screen (live play strips
  the dummy's L and R). The dummy plays whichever build the practice menu gives it. It picks most chips each turn.

## 1. What "state of the art" means here

Current game AIs are trained by reinforcement learning against themselves and a league of their past selves. That is
how AlphaStar, OpenAI Five, GT Sophy and the fighting-game agents since then were made. This design follows it:

- **PPO** with a recurrent core (it remembers what it has seen this round) and an entity encoder (a variable set of
  objects, panels and chips, each a token).
- **A league.** The learner plays its latest self, frozen past versions (picked most often when they still beat it),
  and simple scripted players. Training only against the latest self falls into loops where both sides exploit each
  other's habits.
- **Randomization every round:** the game, both builds, the arena, the seed, the reaction delay, and when the
  custom screen opens. It never sees the same situation twice and can't memorize one.
- **One network for every game and every difficulty.** The difficulty is an input (the reaction delay) plus how it
  picks among its choices, not a separate network.

We need no search at play time. The old fighter searched copies of the battle on every decision: that was slow, and
it reads hidden state. The network acts on what it sees in well under a millisecond on the desktop.

## 2. The parts

```
crates/nettai-battle        + a per-tick log of panels attacked (§5); nothing else changes in the engine
crates/nettai-training      Rust, no Python:
  observe.rs                what a side sees → fixed-size arrays and masks (§3)
  act.rs                    an action → buttons; the custom screen's navigator (§4)
  builds.rs                 random valid builds for any game, from the setup schema (§6.2)
  env.rs                    a round as an episode: delay, rewards, the custom-screen opener (§6)
  pool.rs                   many envs on persistent worker threads, stepped as one batch (§6.4)
  bots.rs                   scripted players: still, mover, masher, shooter, dodger (§9)
  net.rs                    the forward pass, from a weights file (§10)
  ai.rs                     `driver::Ai` for live play (§10)
training/                   Python (uv), never built with the app:
  bridge/                   pyo3 module over nettai-training's pool (its own Cargo workspace)
  nettai_ai/                model, PPO, league, evaluation, export
crates/nettai-training/model/nettai-ai.safetensors   the shipped network, a few MB
```

The bridge has its own Cargo workspace, so `cargo build` of the app never needs Python or pyo3. nettai-training
depends on nettai-frontend only behind its `live` feature (ai.rs, which the app turns on). The bridge leaves it off,
so it builds on a server with no sound or window libraries.

## 3. What it sees

`observe::encode(b, viewer, &mut Obs)` writes one side's view into preallocated arrays. Every read goes through the
per-viewer accessors the renderer uses: `visible_to(r, viewer)`, `telops_for`, `used_chip_for`, `gauge_for`,
`round_end_for`, `sees`. Nothing reads `hands[other]`, `custom.sides[other]`, `setup.players[other]`, the RNG, the
consoles, or the other side's rules state.

**Mirrored.** The field is always laid out from the viewer's side: its own area on the left, x mirrored for side 1,
velocities and facing too. Both sides of a round are the same problem, and the network learns each situation once.

**Tokens.** Each is a fixed-width float vector plus hashed name ids. Unused slots are masked.

| Set | Count | What each holds |
|---|---|---|
| Global | 1 | round mode and fight state, dimming state, gauge (own view), turn, time-up countdown; own HP and max, other's HP (hidden while blind); own charge level and source; the other's B charge if its glow shows; own statuses and timers (paralysis, freeze, bubble, blind, confusion, invincibility, invisibility, barrier and its HP); the other's visible statuses; own chips left; "????" marks on either side; the reaction delay it plays at |
| Panels | 18 | the panel's kind (its name, hashed, and its rule's fields: doubles, damage shift, expires, unbreakable, traps, ...), owner and home (relative), cracked, hole and kind timers, the attack highlight, which sides' attacks and bodies are on it this tick, reserved |
| Objects | ≤ 48 | every actor and attack object `visible_to` the viewer: pool, identity class (virus, field object, navi, player), relative side, its kind's key (hashed), panel and sub-panel position, velocity, HP and max, damage, element, collision flags (own/other attack, body, blocker, floating, breaks guard, pierces invisibility), the panels its region covers, status bits and timers, barrier |
| Attack log | 18 × 8 | for each panel, which side's attacks were presented on it in each of the last 8 ticks (§5) |
| Chips | ≤ 6 + 12 + 1 | own hand in the fight (next chip first, with damage and bonuses as shown); the custom screen's slots, their state and the cursor (§4.2); the other side's last used chip as its telop showed it, and how long ago |
| Rules state | ≤ 24 | own side's content-declared state and game stats, one token per scalar field: the field's name (hashed), its value, its type |
| Forms | ≤ 12 | own form and the other's form as it appears; on the custom screen, the forms its window offers (`form_list`) |

**Chips** are described by what the content states: damage, element, class, codes, MB, the flag and trait bits,
trap kind, modifier kind, damage formula kind, hit strength, lockout, and how it's used (action, cut-in, hand-off,
instant). Its key is hashed into a learned embedding. Keys are hashed without the game, so `cannon` is the same
token in every game that has one.

**Names are hashed, not listed.** Panel kinds, object kinds, chips, forms and rules fields become ids through a
fixed hash into 8,192 buckets. No table of names is kept anywhere. A new game's names land in buckets the network
has seen less of, and it falls back on the numeric facts beside them.

**Fairness is tested, not just intended.** For random positions in all three games, a property test changes
everything the viewer must not see and checks that its observation stays the same byte for byte: the other side's
hand, folder order and screen, the RNG, an object hidden from the viewer.

## 4. What it presses

### 4.1 In the fight

Each decision chooses one of 20 joint actions: a direction (none, up, down, toward, away) × A held or not × B held
or not. They are held until the next decision, every 4 ticks by default (`K`, a setting; Hard may use 2). A press
is "held, then not held", and a charge is "B held" over many decisions. The back special and charged chips fall out
of these sequences. It never presses L, R, START or SELECT in the fight.

**Reaction delay.** The decision at tick t is made from the observation of tick t − D. Observations are encoded
every tick into a ring, so the delay costs nothing. In training D is drawn from 6 to 30 ticks for each round and is
part of the observation. One network then plays every reaction speed.

### 4.2 On the custom screen

Learning the screen's cursor one button at a time would waste most of training on menu navigation. The network
instead points at what it wants, and a navigator presses the buttons to get there:

- **A slot.** One of the 12 screen slots (chips, OK, the rules' buttons such as re-deal or Beast Out). A slot is
  allowed if its state is Selectable or it is a button. The navigator walks the cursor there along the rules' own
  layout (`Screen::target_from`, as the old picker did) and presses A. A pick that changes nothing (EXE4 refuses
  some) is masked for the rest of that screen.
- **A key.** UP, DOWN, LEFT, RIGHT, A, B or START, pressed once. This covers whatever a game's rules put behind a
  key: EXE6's Cross window opens on UP, and inside a window the keys move and choose. While a window is open
  (`Phase::Window`) only keys are allowed, and its offered forms are in the observation.
- **Keys that open something.** When the screen opens, the navigator presses each key once in a copy of the battle
  and notes which ones open a window. Only its own screen is read from the copy. The answer is part of the
  observation, so a Cross window is found from the first round rather than by luck.

Each chip slot also says what picking it would add: whether it completes a program advance (`builder::build` run on
the would-be selection), and the hand as OK would build it, with modifiers folded into the chip before them.

## 5. The engine's part

One addition, generic and outside the simulation:

- **A per-tick log of attacked panels.** Hit regions that live one tick spawn, hit and vanish inside that tick and
  leave nothing behind to read. `Collision` records, for each side, a 40-bit mask of the panels any attack was
  presented on during the current tick. It is cleared at the tick's start, like `panel.highlight`. It is not
  digested, saved for netplay or drawn: like the highlight, it is a look at what happened, and the battle never
  reads it back.

Two costs to check:

- `Content::hash` walks all content on every `Battle::new`. A reset makes one battle, so if the walk is slow the
  pool keeps the hash.
- `pick::live` and `folders::random_folder` call Luau dozens of times. Builds are made ahead of time on the workers.

## 6. The environment

### 6.1 An episode is a round

A round starts at its first custom screen and ends at a KO, at time-up and its judgment, or after 5 minutes of
fight (cut off, with the value carried over). Intros, banners and fades run without decisions. Every round is at
max power (HP 1000 and max levels, as every player's build is).

### 6.2 Builds

`builds::random(content, rng)` makes a valid build for any game from what the game's rules state:

- a navi from the game's list;
- each setup field the rules offer choices for (`facts::offered`), picked at random: version, forms, presets;
- a folder (`folders::random_folder`, which leans toward two codes as people's folders do);
- the rules' `validate` and `build_check` asked, retrying until they pass.

NaviCust layouts and patch cards are picked from the rules' presets where a game has them, and otherwise left at
the game's default.

### 6.3 Who opens the custom screen

In practice only the player opens it, so neither side learns to. The env opens it for one side or the other a
random while after the gauge fills: 0 to 10 seconds, weighted toward soon. The practice options that change the
battle (full gauge, the chip draw) are also turned on in some rounds, since the dummy meets them.

### 6.4 Rewards

- **The round's result:** +1 for a win, −1 for a loss, 0 for a draw. Time-up counts as its judgment says.
- **HP difference as it happens**, scaled by 1/1000 and weighted down over training: (HP the other lost − HP it
  lost).

Nothing else. Rewards for particular behaviors ("use chips", "don't pace") get exploited; strength comes from
winning.

### 6.5 The pool

- `Pool` runs N envs on persistent worker threads. A thread keeps its Luau runtime, which takes about 1 s to load,
  for its whole life.
- `step(actions)` advances every env to its next decision and writes observations, masks, rewards and episode ends
  into numpy arrays the Python side owns, with no copying.
- Each env has both sides' seats; Python says which policy plays each seat.
- Cost, from measured ticks: about 18 µs a tick in EXE6 and 55 µs in EXE5, so at 4 ticks a decision a core makes
  roughly 5,000 to 14,000 decisions a second.

### 6.6 The bridge's interface

The contract between the Rust pool and the Python trainer. Both sides are written against it. Widths come from
`spec()` and are never hard-coded in Python.

```python
import nettai_gym

pool = nettai_gym.Pool(
    games=["exe6", "exe5", "exe4"],  # each env draws one per round
    envs=256, threads=8, seed=1,
    content=None, packs=None,        # default: the repo's content/, $NETTAI_PACKS or data/
    k=4,                             # ticks a fight decision is held
    delay=(6, 30),                   # reaction delay in ticks, drawn per round per seat
    max_fight_ticks=18000,
)
spec = pool.spec()
# {"hash": "<hex>", "buckets": 8192, "fight_actions": 20, "screen_actions": 19,   # 12 slots, then 7 keys
#  "sets": {name: {"count": C, "width": W, "ids": I}, ...}}                      # sets in a fixed order
buf = pool.buffers()   # numpy views of memory the pool owns, refilled by every step()
# per set:  buf["x"][name]    float32 [E, 2, C, W]
#           buf["ids"][name]  int32   [E, 2, C, I]   hashed name ids, 0 = none
#           buf["mask"][name] bool    [E, 2, C]      true = token present
# buf["phase"]        int8    [E, 2]   0 no decision this step, 1 fight, 2 custom screen
# buf["legal_fight"]  bool    [E, 2, 20]
# buf["legal_screen"] bool    [E, 2, 19]
# buf["hp_gain"]      float32 [E, 2]   (HP the other lost - HP it lost) / 1000 since this seat's last decision
# buf["done"]         bool    [E]      the round ended; the obs is already the next round's first
# buf["truncated"]    bool    [E]      ended by max_fight_ticks
# buf["result"]       int8    [E, 2]   +1 / -1 / 0 for the round that ended (valid where done)
# buf["game"]         int8    [E]      index into games
# buf["delay"]        int16   [E, 2]
pool.step(actions)                    # int32 [E, 2]; read only where phase != 0
pool.set_seat(envs, seat, controller) # "policy", "still", "mover", "masher", "shooter", "dodger"; from the next round
pool.set_delay(envs, seat, ticks)     # pins the delay (evaluation); None draws again
pool.round_stats()                    # list of dicts for rounds ended since the last call (§9's measures)
```

- A step advances each env until one of its policy seats needs a decision.
- A seat played by a scripted player always has phase 0.
- Rounds reset on their own, with new builds and a new arena.
- Rewards are put together in Python from `hp_gain` and `result`, so their weights can change during training.

## 7. The network

- **Encoder.** Each token set has its own small MLP to width 128, plus its hashed name's embedding. Two transformer
  layers mix the tokens, and an attention pool reads them out.
- **Memory.** A GRU of 256 carries the round. It remembers chips the other side used, its habits, and how long since
  the last custom screen.
- **Heads.**
  - The fight's 20 actions.
  - The custom screen's pointer: a score for each slot token and each key, masked to what is allowed.
  - A value.
- **Budget.** Each decision may cost at most about 3 million multiply-adds. That is under half a millisecond on the
  desktop and a few milliseconds in the browser's WebAssembly, which has no SIMD in our build today. The budget
  sets the widths; training tries sizes within it.

## 8. Training

- **PPO.** Clipped, with GAE (λ 0.95) and γ 0.997 per decision, about 20 seconds of horizon at 4 ticks a decision.
  Long-ish sequences go through the GRU. It uses the GPU for updates and batched inference, and Rust for
  everything else.
- **League.**
  - About 50% of rounds against its latest self.
  - About 35% against frozen past versions. A version joins the pool every few updates and is picked more often
    while it still beats the learner.
  - About 15% against the scripted players.
  - If the learner plateaus, exploiters train only to beat it, as AlphaStar did.
- **Games.** Every loaded game, mixed in each batch.
- **Logs and checkpoints.** TensorBoard (viewed over an ssh tunnel), and a checkpoint every N updates, kept with the
  run's settings and the git commit.
- **Export.** `nettai_ai.export` writes safetensors with the architecture and an **observation spec hash** in its
  header. The Rust side refuses weights whose hash doesn't match its own encoder, so a changed observation can't
  silently play an old network.

## 9. How we know it's good

All of these run in Rust on the pool, fast, and print a table:

- **Against scripted players, every game:** stands still, moves around, masher (random presses), shooter (walks to
  the dummy's row, shoots and uses chips there), dodger (steps every 40 ticks). Measured: wins, HP taken and lost a
  minute.
- **Behavior:**
  - hits taken from the shooter;
  - share of attack chips that land;
  - chips picked per screen;
  - steps back to the panel just left within a third of a second (pacing);
  - forms taken.
  These are the old fighter's measures, so the two compare.
- **Elo** across the league's versions, mirrored seats. The shipped network is the best by Elo against a fixed
  set.
- **A game it never trained on.** Train on EXE5 and EXE6 only, then play EXE4. This is the test of "works on any
  game". It is run once the main recipe works.
- **The user plays it.** The decisive test.

## 10. In the app

- `nettai_training::NetAi` implements `driver::Ai`. It encodes the observation on every `fight` and `custom_screen`
  call and decides every K ticks from the observation D ticks back.
- **Threads.** On the desktop it runs on the battle's thread, well under a millisecond a decision. In the browser
  it runs inline too if the budget holds. Otherwise it moves to a worker, with the reaction delay as the time it has
  to answer.
- **Weights.** One file, `model/nettai-ai.safetensors`, built into the crate (`include_bytes!`), the same on desktop
  and web. It is replaced only when a better one is trained.
- **Practice menu.** The dummy's Action gets "Fights" back, with its Level row (these rows existed before
  cc24c9a58).

  | Level | Reaction | Choice |
  |---|---|---|
  | Easy | 400 ms | samples widely, sometimes a random action |
  | Normal | 270 ms | samples from its policy |
  | Hard | 170 ms | takes its best action |

  The numbers are set by measuring the levels against each other and the scripted players.

## 11. The cloud machine

- **Shape.** Simulation is the bulk of the work: the engine and its Luau, on CPU cores. The network is about 3
  million multiply-adds a decision, so its updates are small next to that. The cost is cores.
  - **First choice: a CPU-only machine with many cores**, such as a 48–96 core dedicated server, at well under $1 an
    hour.
  - **A GPU only if measured to help.** The first runs time the network updates against the simulation. A GPU
    (RTX 4090 / L4 class, $1–3 an hour with its cores) is added only if the updates turn out to hold training back.
  - **TPUs don't fit** (the user asked, 2026-10-23). They are built for very large matrix work, and this network is
    tiny. XLA handles a recurrent core with changing masks badly, and they would leave the simulation, the real cost,
    where it was.
- **Run size.** A full run is estimated at 2 to 5 billion decisions, about 5 to 15 hours on such a machine. Tuning
  will take several runs.
- **What of the packs goes there: the timings only** (the user: "you don't need the images, just the timing data").
  The battle's loader reads a pack's `content.toml`, `assets.toml`, and each sprite's `animations.json` and
  `sprite.json`. These are the asset names, the animations' timings and the frames' part layouts: a few hundred KB a
  game. No picture or sound is read or sent. `training/cloud/timings.sh` copies just those files out of `data/`. The
  made-up test timings would train a network for a game that isn't quite the real one.
- **Running.** `training/cloud/setup.sh` installs Rust, uv and the bridge on a fresh Ubuntu machine. A run goes in
  tmux; checkpoints and logs are rsynced back.

## 12. Steps

1. **Engine and environment** (crates/nettai-battle, crates/nettai-training):
   - the attack log;
   - observe, with its fairness test;
   - act, with the navigator;
   - builds, env, pool and bots.

   Checked by unit tests on the test content of all three games, and a throughput measurement on the packs.
2. **Trainer** (training/), alongside step 1 against the spec in §3–4: the model, PPO, the league, export, and
   evaluation calls into the pool.
3. **Bridge and smoke run.** The pyo3 module. A 30-minute local run of EXE6 that must clearly beat the masher.
4. **The app.**
   - net.rs, checked against the Python export on recorded observations;
   - `NetAi`;
   - the practice rows;
   - the wasm32 type-check.
5. **Cloud runs.** The full recipe, then the held-out-game test, then the levels' calibration. The weights go in.
6. **If chips are misjudged:** chip fingerprints. Each chip is run once in a sandbox battle to record which panels
   it hits, when, and for how much, as extra features. This is generic, and it costs a second or so for a folder's
   chips at battle start.

## 13. Risks

- **Forms and windows.** If it rarely learns to open the Cross window or use Beast Out, the key probe (§4.2) is the
  first lever, and a small bonus for exploring windows early in training is the second.
- **Practice settings far from training.** A full gauge every turn, for example. These are randomized in (§6.3).
- **Cost of EXE5 and EXE4 ticks.** They are three times EXE6's. Batches are balanced so EXE6 doesn't dominate.
- **Weights in git.** A few MB a time, replaced rarely.
