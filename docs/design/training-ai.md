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
crates/nettai-battle        + Battle::attacked_panels, a per-tick log of panels attacked (§5)
                            + Content keeps its hash once computed (§5)
crates/nettai-training      Rust, no Python:
  spec.rs                   the observation's layout: sets, counts, widths, named features; its hash (§3)
  names.rs                  names as two hashed ids (§3)
  observe.rs                what a side sees → fixed-size arrays and masks (§3)
  act.rs                    an action → buttons; the custom screen's navigator (§4)
  builds.rs                 random valid builds for any game, from the setup schema (§6.2)
  env.rs                    a round as an episode: delay, rewards, the custom-screen opener (§6)
  pool.rs                   many envs on persistent worker threads, stepped as one batch (§6.5)
  bots.rs                   scripted players: still, mover, masher, shooter, dodger (§9)
  stats.rs                  a round's measures, and a table of them (§9)
  examples/throughput.rs    decisions a second on the packs (§6.5)
  net.rs                    the forward pass, from a weights file (§10; a later step)
  ai.rs                     `driver::Ai` for live play (§10; a later step)
training/                   Python (uv), never built with the app:
  bridge/                   pyo3 module `nettai_gym` over nettai-training's pool (its own Cargo workspace), smoke.py
  nettai_ai/                model, PPO, league, evaluation, export
crates/nettai-training/model/nettai-ai.safetensors   the shipped network, a few MB
```

The bridge has its own Cargo workspace, so `cargo build` of the app never needs Python or pyo3. nettai-training
depends on nettai-frontend only behind its `live` feature (ai.rs, which the app turns on). The bridge leaves it off,
so it builds on a server with no sound or window libraries. Without it, nettai-training depends on nettai-battle,
nettai-match, nettai-netplay (the tick input, the masher) and nettai-content (loading a game's pack).

## 3. What it sees

`observe::encode(b, viewer, names, extra, &mut Obs)` writes one side's view into preallocated arrays. Every read goes
through what the viewer's console shows: the objects `visible_to` it, the panels, its own HUD (`gauge_for`,
`telops_for`, `used_chip_for`, `chip_hud_for`), its own custom screen, hand, navi stats and rules' state, and of the
other side what shows on the field (its navi's HP and form while it is seen, its B charge's glow). Nothing reads
`hands[other]`, `custom.sides[other]`, `setup.players[other]`, the RNG, the consoles, or the other side's rules state.
`extra` is what the player knows that the battle doesn't hold for it: its reaction delay, the last ticks' attacks, the
key probe's answer, the slots its screen refused, its decisions on the screen.

**Seen.** An object is seen when it is shown to the viewer this tick, or it blinks (flashing or invisible: shown two
ticks of four) on a side the viewer isn't blind to. A person sees a blinking navi; without this, a decision every 4
ticks could fall on its hidden ticks every time.

**Mirrored.** The field is always laid out from the viewer's side: its own area on the left, x mirrored for side 1,
velocities, sub-panel offsets, facing and regions too. Both sides of a round are the same problem, and the network
learns each situation once.

**Tokens.** Each is a fixed-width float vector plus hashed name ids. Unused slots are masked, and a slot not there is
all zeros. Widths as built (`spec.rs` names every feature; `spec()` lists them):

| Set | Count | Width | Ids | What each holds |
|---|---|---|---|---|
| general | 1 | 71 | own navi, other navi | round phase (custom screen, fight, dimmed, between), own screen's phase, gauge (own view) and full, turn, late turns and their timer, fight time; own HP, max and share; the other's (whether shown); own charge level, source, counter; the other's charge level if its glow shows; own stats (attack, rapid, charge, custom, mega, giga, Regular memory, base HP); chips left in hand and folder, Regular pending; both telop blocks (shown, the other's, "????", damage); the other's used chip shown and how long; the reaction delay; the key probe (done, and the 7 keys); own screen (picks, form chosen, open, decisions, a trade or hold, only OK left); both navis seen, blind |
| panels | 18 | 46 | type | place; its type's rule (doubles and its element, damage shift, expires, submerges, unbreakable, traps, cleared by, carries, carries by move, solid, flags); owner, home, stolen; cracked, visible, hole and type timers, blink; the attack highlight; own and other bodies, floating, blocker, neutral object, own and other attacks registered now; and the attack log (§5): own and other attacks presented in each of the last 8 ticks |
| objects | 48 | 119 | kind, identity, action | every object seen, actors first, then attacks, then effects: pool, identity class (virus, field object, navi, player, none), side (own, other, neutral), own or other player, panel and next panel, sub-panel offset, velocity, facing toward the other side, HP (has, value, max, share), damage, element and its secondary bits, collision (has, own/other attack, own/other body, other bodies, neutral, floating, blocker, breaks guard, pierces invisibility, ground only), the panels it is registered on, 21 status bits, status timers (own side's only), barrier (and its HP, own side's only), actor (charge, B's, lockout, chips held where shown), navi action (the engine's 11), animation |
| chips | 7 | 176 | chip | 6 own hand (next first), then the other's last used chip as its telop named it. Each: its role, hand place; what the record says (element, class, this copy's code, how it is used, the flag, extra flag and trait bits, trap kind, modifier kind, damage formula kind, damage, MB, hit strength, lockout, a Program Advance's result, a link navi's own chip, codes); in hand, damage as shown, bonus and marks; the used chip's damage, bonus, x2; and 40 for the chip's fingerprint (§12 step 6; zeros until then) |
| slots | 12 | 176 | chip (or a button's name) | the custom screen's 12 slots in the screen's order, what its pointer points at (the trainer's requirement: a set of their own, named `slots`), with the chips' features: the slot, its chip's record, the slot's kind and state, cursor, pick order, button uses, marks; the pointer (legal, refused, steps away); what picking it would add (§4.2) |
| rules | 96 | 14 | field, value | own navi stats of its game's own, then own rules' state: one token a scalar (a record's scalars one level deep, an array's elements each, lists left out; EXE6 has 43, EXE5 79, EXE4 49), its source, type (bool, number, enum, reference, code, other), value (set, /100, signed log, enum index), its place in an array |
| forms | 12 | 22 | form | own form, the other's as it shows, and on the custom screen the forms its window offers (`form_list`) and a button's offer (`offer`): role, window place, marked, cursor, chosen; element, weakness bits, base form, buster bonus, chip bonus, null bonus, charged chips |

The attack log is in the panel tokens rather than a set of its own (the coordinator's change: fewer tokens). The
sets' names name the trainer's weights: Python identifiers and no keyword (so `general`, not `global`). The spec's
hash is FNV-1a 64 of the layout's text (every set's name, count, width, ids and features in order, the actions and
the names' hash): `b4dd899d4d9af11a` as built.

**Chips** are described by what the content states: damage, element, class, codes, MB, the flag and trait bits,
trap kind, modifier kind, damage formula kind, hit strength, lockout, and how it's used (action, cut-in, hand-off,
instant). Its key is hashed into a learned embedding. Keys are hashed without the game, so `cannon` is the same
token in every game that has one.

**Names are hashed, not listed.** Panel types, object kinds, identities, actions, chips, buttons, forms, navis and
rules fields become ids through a fixed hash, two ids a name from two seeds, whose embeddings the network sums: two
names that share one bucket almost never share the other. No table of names is kept anywhere. A new game's names land
in buckets the network has seen less of, and it falls back on the numeric facts beside them. The hash, which the
Rust player and the Python trainer must agree on for as long as a network is played:

```
id_k(name) = 1 + (fnv1a64(SEEDS[k], utf8(local(name))) mod 8191),  k = 0, 1;  0 = no name
fnv1a64(h, bytes): for each byte b: h = (h xor b) * 0x100000001b3 (mod 2^64)
SEEDS = [0xcbf29ce484222325, 0x84222325cbf29ce4]
local(name): the name without its root ("exe6:cannon" is "cannon")
```

`nettai_gym.name_ids("cannon")` is `(834, 2843)`. An engine's own action has no key: its variant's name is hashed.

**Fairness is tested, not just intended.** For 288 positions of the three games' test content (mashers and shooters
playing, on the screen and in the fight), a property test changes everything the viewer must not see and checks that
its observation stays the same byte for byte: the other side's hand, folder, custom screen, setup and rules' state,
the battle's and both consoles' RNG, and every object hidden from the viewer (but its own navi, whose HP its HUD
shows).

## 4. What it presses

### 4.1 In the fight

Each decision chooses one of 20 joint actions: a direction (none, up, down, toward, away) × A held or not × B held
or not, numbered direction × 4 + A × 2 + B. Toward is right for side 0 and left for side 1. They are held until the
next decision, every 4 ticks by default (`k`, a setting; Hard may use 2). A press is "held, then not held", and a
charge is "B held" over many decisions. The back special and charged chips fall out of these sequences. It never
presses L, R, START or SELECT in the fight. Every action is always allowed.

**Reaction delay.** The decision at tick t is made from the observation of tick t − D. Decisions come at the env's
ticks with t mod k = 0 while the fight runs (dimmed too: a counter cut-in is pressed then), so the observations a
decision can need are those of ticks s with (s + D) mod k = 0: the env encodes just those, into a ring, one encoding
every k ticks a seat. In training D is drawn from 6 to 30 ticks for each round and seat and is part of the
observation. One network then plays every reaction speed.

### 4.2 On the custom screen

Learning the screen's cursor one button at a time would waste most of training on menu navigation. The network
instead points at what it wants, and a navigator presses the buttons to get there:

- **A slot.** One of the 12 screen slots (chips, OK, the rules' buttons such as re-deal or Beast Out). A slot is
  allowed while the screen is choosing, if it is a chip it can still pick (selectable, fewer than 5 picks), a button
  with uses left, or OK, the cursor can reach it, and the screen hasn't refused it. The navigator walks the cursor
  there along the rules' own layout (`Screen::target_from`), a direction held two ticks (the screen moves on the
  second, its auto-repeat's) and let go one, and presses A, held a tick and let go a tick. A pick after which the
  screen is the same (the picks, the slots' states, the phase, the form, the cursor) is refused for the rest of that
  screen (EXE4 refuses a chip in a code it doesn't come in).
- **A key.** UP, DOWN, LEFT, RIGHT, A, B or START, pressed once. This covers whatever a game's rules put behind a
  key: EXE6's Cross window opens on UP, and inside a window the keys move and choose. While a window is open
  (`Phase::Window`) only keys are allowed, and its offered forms are in the observation.
- **Keys that open something.** At the screen's first decision, each key is pressed once in a copy of the battle
  (the other side pressing nothing) and the env notes which ones open a window. Only its own screen is read from the
  copy. The answer is part of the observation, so a Cross window is found from the first round rather than by luck.
- **No delay on the screen.** A screen decision is made from the screen as it is: nothing moves on it but the
  player's own cursor, and the navigator needs it as it is.
- **A screen ends.** After 24 decisions on one screen only OK is allowed (keys stay allowed inside a window), and a
  screen open 3,600 ticks cuts the round off.

Each chip slot also says what picking it would add: whether it completes a Program Advance (`builder::build` run on
the would-be picks), the Program Advance's damage, the damage the hand as OK would build it gains (modifiers folded
into the chip before them), and whether it is a modifier that would stand alone. The would-be hands take each chip's
damage from its record: a damage formula can read what the viewer doesn't see (the other navi's HP while blind).

## 5. The engine's part

Two additions, outside the simulation:

- **A per-tick log of attacked panels**, `Battle::attacked_panels(side)`. Hit regions that live one tick spawn, hit
  and vanish inside that tick and leave nothing behind to read. As a registration whose collision type is an attack
  of a side (`collision::ATTACK_OF`, the type bits 0x80000000 and 0x40000000) is presented, its panels' bits (y × 8 +
  x, as `Collision::masks` index them) are set in that side's 40-bit mask. It is cleared as each tick starts, beside
  the panels' one-frame looks. It is not digested (digest.rs lists it), saved for netplay or drawn, and the battle
  never reads it back: a test scrambles it after every tick of a battle and finds every digest the same.
- **The content's hash kept.** `Content::hash` walked all of a game's content on every call: 52 to 90 ms, twice a
  `Battle::new`, more for a round's setup and a build's checks. `Content` now keeps it once computed
  (`Content::hash_cache`, no part of the content: left out of its hash and equality, forgotten by `define` and
  `rules_mut`, not copied with it). The three games' hashes are unchanged.

`pick::live` and `folders::random_folder` call Luau dozens of times. Builds are made ahead of time on the workers
(§6.5).

## 6. The environment

### 6.1 An episode is a round

A round starts at its first custom screen and ends at a KO or the judge's ruling after time-up (the fight's WIN,
LOSE or DRAW state: the env doesn't wait for the banners and the fade), or is cut off: 18,000 ticks of fight (5
minutes, `max_fight_ticks`), a custom screen open 3,600 ticks, or the engine stopping the round (`RoundEnd::Error`).
Intros, banners and fades run without decisions. A round of a match of one round is a single battle (no set). Every
round is at max power (HP 1000 and max levels, as every player's build is) unless the HP option says otherwise.

### 6.2 Builds

`builds::Maker::random` makes a valid build for any game from what the game's rules state:

- a group's new build (`Builds::new_build`); a navi from the game's list, the navi that changes form half the time
  (most players play it) and otherwise any of them;
- each fact of forms the build chooses (`Builds::form_choices`: the version, a form list) picked at random: one form,
  or a list full half the time and otherwise of a random length; a random preset of each choice (`take_preset`);
- with chance `randomize` (default 0.75), every other fact a build states drawn: not the engine's roles, not those no
  build states (`never_stated`), not a choice's facts, and not a list of other navis (a side's party: the env plays
  no switch to one, and party battles are out of scope for now). A fact is drawn from what its type allows (a flag, a
  variant, a definition of the game's, a small number below 8); a list is filled an entry at a time to a random length
  (16 at most), each entry kept only where the rules' `validate` and `build_check` say nothing of it, asked of the
  side's round set up with its setup block changed in place (they read the setup: no new battle a check); a single
  fact they say something of goes back to its default. The NaviCust and the patch cards come out of this, through
  the rules' own checks, with no knowledge of a board in Rust;
- a folder (`folders::random_folder_of`, which leans toward two codes as people's folders do) of the chips the rules
  let it hold, those held out left out or favored (`chip_holdout`, §6.6); its Regular chip as the draw gives it, and
  tag chips half the time where the rules take them;
- the build checked whole (`build::Checked`, the rules' `validate` and `build_check`), drawn again until it passes.

Measured (release, the packs, 20 builds a game): every build accepted on its first try; 4.3, 4.9 and 3.8 ms a build
at the defaults (EXE6, EXE5, EXE4), 7.4, 9.1 and 8.4 ms with every fact drawn; list entries kept 25, 19 and 23% (NaviCust
pieces the board refuses, mostly).

A round between two builds is a match of one round on a random arena of the game's, both sides' base HP the drawn HP
where the rules take one (`PlayerFact::BaseHp`); a navi whose HP is its level's (a link navi) keeps its own, as the
rules refuse a base HP for it.

### 6.3 Who opens the custom screen

In practice only the player opens it, so neither side learns to. The env opens it for one side or the other a
random while after the gauge fills: 0 to 10 seconds, weighted toward soon (600 u² ticks, u uniform), pressing L
(a press and a let-go until the screen opens). No seat's own L or R reaches the fight.

The practice options that change the battle are on in some rounds, each with chance `practice` (default 0.25), since
the dummy meets them: a full gauge (whenever it runs, it is full), and the folders dealt in their own order. Both are
nettai-frontend's training session's (`Training`), done in the env on the battle's state as the session does them
(the frontend is out of reach without the `live` feature).

### 6.4 Rewards

- **The round's result:** +1 for a win, −1 for a loss, 0 for a draw. Time-up counts as its judgment says. A round
  cut off is 0 for both, with `truncated` set.
- **HP difference as it happens**, weighted down over training: (HP the other lost / the other's max HP − HP it lost
  / its own max HP), so a whole KO adds up to 1 whatever the HP (the coordinator's change from / 1000).

Nothing else. Rewards for particular behaviors ("use chips", "don't pace") get exploited; strength comes from
winning.

### 6.5 The pool

- `Pool` runs N envs on persistent worker threads. A thread keeps its Luau runtimes (a game's takes about 1 s to
  load) for its whole life, and any worker steps any env (a battle is plain data).
- `step(actions)` hands every worker the batch's actions. Each claims the next env not yet stepped, steps it to its
  next decision and writes its rows of the buffers, until none is left: a slow core (an efficiency core), a round's
  start or a key probe holds up no other worker's envs. The buffers are the pool's own memory, never reallocated, and
  the bridge shows them to Python as numpy arrays with no copy.
- Each env has both sides' seats; Python says which policy plays each seat.
- While a worker waits for the next step (Python's forward pass) it makes builds ahead. Each game has its own stream
  of draws for its builds, used in order, so a round's builds are the same however busy the workers were: a pool of
  one thread and a pool of three play the same, buffer for buffer (tested).

Measured on the packs (release, `examples/throughput`, 10 s a measure, both seats a policy taking random allowed
actions, three builds in four with every fact drawn): decisions a second. "One env" runs on the calling thread; its
round start includes making both builds, which a pool makes ahead while Python works. The Mac (an M1 Max: 8
performance and 2 efficiency cores) was busy with other sessions (load 8 to 12), so the many-thread numbers are low;
a quiet machine of uniform cores scales further.

| Game | One env | Pool, 1 thread | Pool, 4 threads | Pool, 10 threads | Tick | Encoding | Round start |
|---|---|---|---|---|---|---|---|
| EXE6 | 23,855 | 20,104 | 47,271 | 63,012 | 20 µs | 10.5% | 13.4 ms |
| EXE5 | 6,073 | 5,569 | 15,021 | 23,827 | 73 µs | 3.9% | 16.4 ms |
| EXE4 | 12,785 | 11,465 | 32,595 | 40,793 | 32 µs | 6.7% | 16.0 ms |

"Encoding" is the share of one env's time spent encoding observations (one encoding every k = 4 ticks a seat, and
each screen decision); the key probes take 2 to 3% (0.5% in EXE4). A round of random play runs 6,000 to 16,000
ticks. The three games mixed, through the bridge (`smoke.py`, 32 envs on 8 threads): 24,036 decisions a second.

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
    hp=(1000, 1000),                 # both sides' base HP, drawn per round (where the navi takes one)
    chip_holdout=(0.0, 0, "exclude"),  # (fraction, seed, mode): chips held out of training, below
    practice=0.25,                   # the chance each practice option is on in a round
    randomize=0.75,                  # the chance a build draws every fact it states (§6.2)
)
spec = pool.spec()
# {"hash": "<hex>", "buckets": 8192, "fight_actions": 20, "screen_actions": 19,   # 12 slots, then 7 keys
#  "screen_keys": ["up", "down", "left", "right", "a", "b", "start"],
#  "chip_tokens": {"hand": (0, 6), "used": (6, 1)},                              # (first, count) in "chips"
#  "fingerprint": 40, "games": [...],
#  "sets": {name: {"count": C, "width": W, "ids": I, "id_names": [...],         # sets in a fixed order:
#                  "features": [(name, width), ...]}, ...}}                     # general, panels, objects,
#                                                                               # chips, slots, rules, forms
buf = pool.buffers()   # numpy views of memory the pool owns, refilled in place by every step(): the same arrays
# per set:  buf["x"][name]    float32 [E, 2, C, W]
#           buf["ids"][name]  int32   [E, 2, C, I]   two hashed ids a name, 0 = none
#           buf["mask"][name] bool    [E, 2, C]      true = token present
# buf["phase"]        int8    [E, 2]   0 no decision this step, 1 fight, 2 custom screen
# buf["legal_fight"]  bool    [E, 2, 20]
# buf["legal_screen"] bool    [E, 2, 19]  the slots set's 12 tokens, then the 7 keys
# buf["hp_gain"]      float32 [E, 2]   HP the other lost / its max HP - HP it lost / its own, everything since this
#                                      seat's last decision (steps where it had phase 0 too); read where the seat
#                                      decides or the round ends
# buf["done"]         bool    [E]      the round ended; the obs is already the next round's first
# buf["truncated"]    bool    [E]      cut off (max_fight_ticks, a screen open too long, the engine stopped)
# buf["result"]       int8    [E, 2]   +1 / -1 / 0 for the round that ended (valid where done)
# buf["game"]         int8    [E]      index into games
# buf["delay"]        int16   [E, 2]
pool.step(actions)                    # int32 [E, 2]; read only where phase != 0; a step with an action not
                                      # allowed is refused whole (an exception), nothing moved
pool.set_seat(envs, seat, controller) # "policy", "still", "mover", "masher", "shooter", "dodger"
pool.set_delay(envs, seat, ticks)     # pins the delay (evaluation); None draws again
pool.set_hp(lo, hi)
pool.set_chip_holdout(fraction, seed, mode)
pool.round_stats()                    # rounds ended since the last call (§9's measures):
                                      # [{"env": int, "ticks": int, "seats": [dict, dict], "game", "fight_ticks",
                                      #   "screens", "truncated", "error", "full_gauge", "folder_order"}, ...],
                                      # a seat's dict: "result" (+1 / -1 / 0), "controller", "navi", "delay",
                                      # "max_hp", "hp_lost", "hits_taken", "chips_used", "attack_chips",
                                      # "attacks_landed", "chips_picked", "steps", "steps_back", "forms_taken"
pool.profile()                        # the envs' time: encoding, round starts, key probes, builds made ahead
nettai_gym.name_ids("cannon")         # (834, 2843): the trainer can check its own hash against the pool's
nettai_gym.holdout_unit(seed, key)    # a chip key's number in [0, 1) with the seed
```

- The first round begins when the pool is made, every seat "policy". `set_seat`, `set_delay`, `set_hp` and
  `set_chip_holdout` take effect from the next round (the one after the round each env is playing).
- A step advances each env until one of its policy seats needs a decision.
- A seat played by a scripted player always has phase 0. An env with no policy seat plays a whole round a step.
- Rounds reset on their own, with new builds and a new arena.
- Rewards are put together in Python from `hp_gain` and `result`, so their weights can change during training.
- `step` lets go of the GIL while the workers run.
- **Chips held out** (the trainer's requirement: one way, in place of a list of chips): a chip is held out where its
  key's number with the seed is below the fraction, among the chips a folder may hold. The number: FNV-1a 64 from
  0xcbf29ce484222325 over the seed's 8 bytes (little-endian), then the key's UTF-8 (its root dropped); its top 53
  bits over 2^53. Mode "exclude": random builds never use them. Mode "favor": folders lean toward them (each held-out
  chip as many times in the draw as the others together over their count: about half a folder's draws), for
  evaluation.

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
