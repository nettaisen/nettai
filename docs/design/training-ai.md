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
  fingerprint.rs            what each chip does, run in a sandbox; pairs of chips that work together (§3.1)
  env.rs                    a round as an episode: delay, rewards, the custom-screen opener (§6)
  pool.rs                   many envs on persistent worker threads, stepped as one batch (§6.5)
  bots.rs                   scripted players: still, mover, masher, shooter, dodger (§9)
  stats.rs                  a round's measures, and a table of them (§9)
  examples/throughput.rs    decisions a second on the packs (§6.5)
  net.rs                    the forward pass, from a weights file (§10.1)
  ai.rs                     `NetAi`, live play's `driver::Ai`, and the fingerprints made on the app's threads
                            (§10.2, §10.3; feature `live`)
  examples/ai_costs.rs      what live play costs: fingerprints, a decision (§10.3, §10.4)
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
| panels | 18 | 47 | type | place; its type's rule (doubles and its element, damage shift, expires, submerges, unbreakable, traps, cleared by, carries, carries by move, solid, flags); owner, home, stolen; cracked, visible, hole and type timers, blink, a stolen panel's blink as it goes back; the attack highlight; own and other bodies, floating, blocker, neutral object, own and other attacks registered now; and the attack log (§5): own and other attacks presented in each of the last 8 ticks |
| objects | 48 | 119 | kind, identity, action | every object seen, actors first, then attacks, then effects: pool, identity class (virus, field object, navi, player, none), side (own, other, neutral), own or other player, panel and next panel, sub-panel offset, velocity, facing toward the other side, HP (has, value, max, share), damage, element and its secondary bits, collision (has, own/other attack, own/other body, other bodies, neutral, floating, blocker, breaks guard, pierces invisibility, ground only), the panels it is registered on, 21 status bits, status timers (own side's only), barrier (and its HP, own side's only), actor (charge, B's, lockout, chips held where shown), navi action (the engine's 11), animation |
| chips | 7 | 250 | chip, changes_to, leaves, kind_1, kind_2, kind_3 | 6 own hand (next first), then the other's last used chip as its telop named it. Each: its role, hand place; what the record says (element, class, this copy's code, how it is used, the flag, extra flag and trait bits, trap kind, modifier kind, damage formula kind, damage, MB, hit strength, lockout, a Program Advance's result, a link navi's own chip, codes); in hand, damage as shown, bonus and marks; the used chip's damage, bonus, x2; and its fingerprint, 114 (§3.1), whose names are the token's ids after the chip's |
| slots | 12 | 250 | as chips (the chip, or a button's name) | the custom screen's 12 slots in the screen's order, what its pointer points at (the trainer's requirement: a set of their own, named `slots`), with the chips' features: the slot, its chip's record, the slot's kind and state, cursor, pick order, button uses, marks; the pointer (legal, refused, steps away); what picking it would add (§4.2); its chip's fingerprint |
| rules | 96 | 14 | field, value | own navi stats of its game's own, then own rules' state: one token a scalar (a record's scalars one level deep, an array's elements each, lists left out; EXE6 has 43, EXE5 79, EXE4 49), its source, type (bool, number, enum, reference, code, other), value (set, /100, signed log, enum index), its place in an array |
| forms | 12 | 22 | form | own form, the other's as it shows, and on the custom screen the forms its window offers (`form_list`) and a button's offer (`offer`): role, window place, marked, cursor, chosen; element, weakness bits, base form, buster bonus, chip bonus, null bonus, charged chips |

The attack log is in the panel tokens rather than a set of its own (the coordinator's change: fewer tokens). The
sets' names name the trainer's weights: Python identifiers and no keyword (so `general`, not `global`). The spec's
hash is FNV-1a 64 of the layout's text (every set's name, count, width, ids and features in order, the actions and
the names' hash): `c4d9e4a61b0f54ba` as built.

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

### 3.1 Chip fingerprints

A chip's name is a learned embedding, which a chip seen rarely (or a new game's) hardly has. Its fingerprint says what
it does, measured: each chip of a game run in a sandbox (`fingerprint.rs`), once a game (computed as a pool starts,
on its threads, kept by content hash in the process and in `cache` on disk; live play makes two folders' chips on threads
of its own and keeps them by content hash, §10.3). Nothing names a chip: a chip is what the battle shows of it. A
chip's fingerprint is its own alone: nothing of another chip goes in it, so it is the same however many of the game's
chips are fingerprinted with it (the coordinator's call, 2026-10-23: live play makes only the chips it may see).

- **The sandbox.** Two of the game's default builds, through the first custom screen with nothing picked, every panel
  the rules' normal type, three columns a side; the user (side 0) on its front column's middle panel holding the chip
  alone, the target (the other navi, pressing nothing) walked to each of the nine panels of its area in turn, 1 to 3
  columns ahead, a row above, level or below. The user presses A; the run lasts until it is quiet (the user idle, none
  of the chip's attacks left, not dimmed) or 180 ticks.
- **Damage net of the setup.** Every run's damage is the target's HP lost less what the same setup costs it with no
  chip used (a control run, made once a setup and panel per thread): a poison panel's own damage, the first chip's
  that goes on, isn't the chip's.
- **Recorded, by target panel:** whether it hit, the damage (0 where it hit nothing), the first hit's tick. **Of the run
  that hit hardest** (the middle one if none hit): the statuses it started on the target, by the engine's status
  timer (paralysis, confusion, blindness, immobility, flash, submersion, invulnerability, freezing, bubble); the
  user's HP and barrier gained; the panels whose type changed (in the user's area, the other's; which, as an 18-bit
  map; the type most turned to, as a name id); the panels whose owner changed (taken for the user, lost; which; whether
  a panel of the other side's front column was taken); the objects left on the field (how many, the user's, the
  other's, the longest-lived's kind as a name id and its ticks); the run's ticks.
- **Conditioned runs**, at the panel it hit hardest (the middle one if none), each the damage as a ratio to the
  plain run's at the same panel, at most 4: a chip that hits nothing plainly and hits under a condition comes out at
  4. The target standing on each panel type the content declares (but the normal role's, the plain runs' own, and the
  missing and broken roles': set as the run starts); the target in each status the content's statuses set (the
  engine's status timers some status of the content's sets, each with the status that sets it longest: EXE6's
  paralysis, confusion, blindness, immobility, freezing and bubble, EXE5's and EXE4's first four), put on it by a hit
  of the engine's own (a hitbox of the user's side on the target's panel doing no damage, of the rules' own collision
  types for a hit that reaches navis, carrying the status: it lands through the target's intake as any attack's
  status does, or the run is left out), then the chip under test while the status holds; the user's front column held
  by the other side (as a steal leaves it: `Battle::set_panel_alliance` and the column's timer, so it would go back as
  a real steal does), the user a column back, which brings out a revenge-type chip (GrabRevenge: nothing plainly, 80
  so); and the front line advanced (the other side's front column the user's, the user on it), the target a column
  farther, which shows a chip's reach growing with the front line. The token keeps the three panel types farthest
  from 1 (named by its ids `kind_1` to `kind_3`), every status's (and whether run), and the two ownership conditions'
  ratio and damage. Before (version 2) a status was put on the target by the first chip of the game that inflicts it
  at that panel, which tied a chip's fingerprint to the whole game's and let the inflicter's own leftovers count (a
  bubble's then Wind at 4); a status no chip of the game inflicts alone (EXE6's freezing, which aqua on ice brings)
  wasn't run at all.
- **A chip that arrives through a form** is run with that form the side's choice at OK, as the rules have it when
  they put the chip in the hand (`chip_forms`: a form whose definition names the chip in a field of its rules'
  extension of forms typed as a chip, no field named: EXE4's souls' chips, which OK puts first in the hand for the soul
  given, their use the change into it). Given alone, all twelve stopped the engine in every run ("EXE4's soul change
  asked for no soul": their change reads the soul given at OK, and none was), which no player can bring about: no
  folder may hold them (EXE4's folder rules take Standard, Mega and Giga chips; they are Special), and the UNITE button
  that puts one in the hand gives its soul with OK (tested: `a_chip_that_arrives_through_a_form_is_in_no_folder`). Not
  a port bug: the sandbox's setup was one the game never has.
- **Conditions that tell nothing** (`fingerprint::uninformative`, `Fingerprints::uninformative`): a condition whose
  ratio is the same, to 0.01, for at least 95% of the chips it applies to (those that hit in the plain run at its
  panel or under it: a chip that hits in neither is at 1 under every condition), unless a pair is made of it. They are
  said, no longer dropped from the fingerprints: which they are is the whole game's to say, and a chip's fingerprint
  is its own alone; a feature nearly every chip has the same value of tells the network nothing either way. On the
  packs: EXE6's cracked panels, confusion, blindness and immobility; EXE5's cracked and magnet panels and its four
  statuses; EXE4's holes, metal and its four statuses.
- **Layout** (`spec()`'s `fingerprint_features`): positions 27 (9 × hit, damage/400, first hit/180), statuses 9, user
  2, panels 2, owners 3, left 4, duration 1, changed 18, owner_changed 18, status_ratios 9, status_conditioned 9,
  kind_ratios 3, kind_conditioned 3, stolen 3, advanced 3: 114. Ratios are kept over 4.
- **Pairs.** A pair (setup, payoff) is a chip whose runs brought a condition (turned the panel under the target to a
  type, started a status on it, took a panel of the other side's front column) and a chip whose damage that condition
  raises past 1.3 times (grass then fire, a bubble then elec, AreaGrab then a sword that reaches the target only from
  the stolen column). Found on the packs (version 3), by the kind of effect the setup brings: EXE6 298 (a panel type
  made 193: grass 180, ice 12; a status 69: bubble 39, paralysis 30; ownership 36), EXE5 156 (panel 84: sea 68, grass
  14; ownership 72), EXE4 149 (panel 80: cracked 36, grass 36, among them FireSoul's change, which grows grass; ownership
  69); 328, 160 and 133 in version 2. The status pairs version 2 had and 3 hasn't were the inflicter's leftovers (a bubble
  then Wind, AirShot or the tank cannons; PoisonSeed then a paralysis); version 3 has a bubble then ElecSword at 2,
  which the inflicter's own hit hid. Freezing now raises 23 EXE6 chips 1.33 to 2 times (GolemHit, DrillArm, IronShell,
  FlameHook...), but no chip freezes the target alone, so no pair is of it. GrabRevenge's stolen column is at 4 (80, 100
  in EXE4: nothing plainly); no pair is of it, no setup of the user's bringing it. No pair is of objects left: no
  conditioned run measures one. Random folders are seeded with them (§6.2).
- **Cost** (release, the packs, this busy Mac): a whole game on 10 threads in 1.4 s (EXE6, 9,585 runs: 27 a chip), 10.1
  s (EXE5, 8,602), 3.7 s (EXE4, 8,184); both folders' chips on one thread and on four: §10.3. Every chip of the three
  games, the packs' and the test content's, runs with no run the engine stops (tested on the test content:
  `every_chip_of_every_game_runs_without_a_stop`).

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
- with chance `combos` (default 0.3), the folder seeded with one of the game's pairs (§3.1) whose chips the side may
  both hold, drawn by the kind of effect its setup brings first (a panel type made, a status, ownership taken), then
  within it, so no kind's many pairs crowd out the rest: each chip in a code both come in (or `*`), two copies each
  where the rules take them, the rest drawn as usual. Of 300 folders all seeded: EXE6 panel 68, status 98, ownership
  134 (drawn by pair, grass alone took more than half); EXE5 114, 50, 136; EXE4 80, 128, 92 (its two status pairs
  fill all of theirs). The round's stats say what came of it (§9);
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
actions, three builds in four with every fact drawn, three folders in ten seeded with a pair, the fingerprints in the
views): decisions a second. "One env" runs on the calling thread; its round start includes making both builds, which a
pool makes ahead while Python works. The Mac (an M1 Max: 8 performance and 2 efficiency cores) was busy with other
sessions (load 8 to 12), so the many-thread numbers are low; a quiet machine of uniform cores scales further.

| Game | One env | Pool, 1 thread | Pool, 4 threads | Pool, 10 threads | Tick | Encoding | Round start |
|---|---|---|---|---|---|---|---|
| EXE6 | 21,846 | 17,549 | 41,870 | 60,307 | 18 µs | 13.9% | 13.6 ms |
| EXE5 | 5,709 | 5,184 | 15,851 | 25,478 | 73 µs | 5.0% | 15.5 ms |
| EXE4 | 12,267 | 10,381 | 30,299 | 49,759 | 35 µs | 8.3% | 14.9 ms |

"Encoding" is the share of one env's time spent encoding observations (one encoding every k = 4 ticks a seat, and
each screen decision); the key probes take 2% (0.4% in EXE4). A round of random play runs 6,000 to 16,000 ticks. The
three games mixed, through the bridge (`smoke.py`, 32 envs on 8 threads): 24,964 decisions a second.

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
    combos=0.3,                      # the chance a random folder is seeded with a pair of chips (§3.1, §6.2)
    cache=None,                      # a directory the chips' fingerprints are kept in, by content hash
)
spec = pool.spec()
# {"hash": "<hex>", "buckets": 8192, "fight_actions": 20, "screen_actions": 19,   # 12 slots, then 7 keys
#  "screen_keys": ["up", "down", "left", "right", "a", "b", "start"],
#  "chip_tokens": {"hand": (0, 6), "used": (6, 1)},                              # (first, count) in "chips"
#  "fingerprint": 114, "fingerprint_features": [(name, width), ...], "games": [...],
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
                                      # "attacks_landed", "chips_picked", "steps", "steps_back", "forms_taken",
                                      # and for a folder seeded with a pair: "pair" (setup, payoff), "pair_picked",
                                      # "pair_ordered", "pair_used", "payoff_damage" (None where not seeded)
pool.profile()                        # the envs' time: encoding, round starts, key probes, builds made ahead
pool.pairs()                          # {game: [(setup key, payoff key, condition, ratio), ...]}
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
  These are the old fighter's measures, so the two compare. The env measures them every round (`stats.rs`,
  `round_stats()`), and `stats::Table` sums them by game and pairing.
- **Combos** (the user: multi-step setups learned well): for each folder seeded with a pair (§3.1, §6.2), whether both
  chips were picked, picked on one screen setup first, used in that order within 5 seconds, and the most the payoff
  did (the other navi's HP lost within 2 seconds of its use) as a ratio to its plain fingerprint's best. Rates of
  these against the scripted players, by condition (a panel type, a status, the front line), say whether it learned
  the setups.
- **Elo** across the league's versions, mirrored seats. The shipped network is the best by Elo against a fixed
  set.
- **A game it never trained on.** Train on EXE5 and EXE6 only, then play EXE4. This is the test of "works on any
  game". It is run once the main recipe works.
- **The user plays it.** The decisive test.

## 10. In the app

As built (branch learned-ai-app): the forward pass (`net.rs`) and the player (`ai.rs`, feature `live`, the only
place nettai-frontend is used) in nettai-training. The practice menu's rows come back with the first trained
weights (§10.6).

### 10.1 The forward pass

- **The file.** `Net::from_safetensors(bytes)`: safetensors read by a reader of its own (an 8-byte header length,
  the header through serde_json; no new dependency). Refused, each with the reason: a format other than `nettai-ai`
  version 1; a `spec_hash` other than the encoder's (`spec::hash()`); an architecture field it doesn't know (a new
  knob of the trainer's is an error until the pass computes it); a tensor missing, of another shape, or one it
  doesn't read (a strict load); an activation other than ReLU; actions or name buckets other than the encoder's.
  Every width comes from the `architecture` metadata. The network's sets are found in the encoder's observation: a
  set of the encoder's own, or a part the trainer cut from one (`split`: `chips` read as `hand`, tokens 0–5, and
  `used`, token 6), covering every token of every encoder set once.
- **The pass** (`Net::forward`), as training/MODEL.md has it: each present token normalized and clipped, its set's
  MLP, its set's vector and its names' embeddings; the transformer layers over the present tokens of the sets not
  pool-only; the final LayerNorm on every present token; the attention pool and the one-token sets read directly;
  the GRU (gates r, z, n); the heads of `core = [h', z]`. Absent tokens are never computed. A decision computes the
  heads it reads (`Heads::Fight`; `Heads::Screen`, the keys and the pointer, whose core part is computed once;
  `Heads::All` adds the value, for the parity test). A player's `Memory` holds the GRU state and room for a pass, so a
  decision allocates nothing.
- **The loops.** Plain f32, no threads, no unsafe: a dot product of a weight row in eight independent sums over
  fixed-size chunks (`as_chunks::<8>`), which LLVM turns into vector lanes (a single sum is a chain it may not
  reorder). The repo's nightly toolchain builds them 1.8 times faster than stable 1.92 does.
- **Choosing** (`net::choose`), from the level's knobs in the weights' `levels`: the allowed actions are those legal
  with a finite logit; greedy takes the first of the largest; otherwise with chance `epsilon` one of them uniformly,
  else a sample of softmax(logits / temperature).
- **Parity** (`net::tests`). `tests/fixtures/net.safetensors` and `net.fixture.safetensors` (1.2 MB together) are
  made by `tests/fixtures/make_net_fixture.py` with the trainer's own `Model` and export (`save_weights`,
  `save_fixture`) on the real pool's spec: a network of small odd widths (d 8, two heads, two layers, GRU 10),
  every parameter and the input normalization drawn wide so the outputs vary, and 15 decisions recorded from the
  real pool on the packs, EXE6, EXE5 and EXE4, custom screens and fights (up to 118 tokens present, 15 objects), the
  GRU carried from a random h0 and reset at each game. Every finite output matches within 1e-4 and every −∞ exactly:
  the raw and masked logits, the value and the GRU state after each decision. The trainer's default widths (d 64,
  four heads, one layer) were checked the same way once (`--size default`), not committed: its name table alone is 2
  MB. Weights of another spec, format version, an unknown architecture field or a missing tensor are refused
  (tested).

### 10.2 The player

`NetAi::new(weights, level, seed, fingerprints)` (the weights as bytes, so the app can `include_bytes!` the shipped
file; `NetAi::of` takes a network already read, for an app that keeps one); `Level` is Easy, Normal or Hard, its
knobs the weights' by name. It plays a side as the env plays a policy seat (§4, §6), once a tick on the battle's
thread (`fight` or `custom_screen`, both the same tick of the round):

- It counts ticks from the round's start, keeps the attack log, and encodes the view into its reaction-delay ring
  on the ticks a decision will need (`(t + D) mod k = 0`, `D` the level's delay).
- In the fight it decides every `k` ticks from the view of `D` ticks before (what it sees now while the round is
  younger than that) and holds the action till the next decision.
- On the custom screen it decides whenever its navigator is idle and the screen reads keys, from the screen as it
  is, the keys probed in a copy of the battle as each screen first reads them, only OK after 24 decisions, the
  refused slots masked; each new screen's navigator starts from nothing. All in the env's order.
- `k` (4) and the 24 decisions are the env's defaults, as trained: the weights don't state them. If training ever
  uses another `k`, the levels' metadata should carry it.
- It never reads `theirs`: it doesn't hear the player's buttons (tested: the same play whatever it is told).
- The GRU state carries through a round and starts over at the next (a new round is a new score of the set);
  `reset`, as the set starts over, starts everything over, its draws too (tested: the same buttons again).
- `error()` says why it stopped, where it did (its fingerprints failed or are of another content, a decision with
  nothing allowed): it presses nothing from then on.
- Its fingerprints are made as it needs them, and a decision whose view lacks one waits for it (§10.3).

### 10.3 Its chips' fingerprints

**The chips it may see, as they are needed.** A chip's fingerprint is its own alone (§3.1, version 3), so a chip
made apart is what the pool makes of it (on the packs, of a round's both folders' chips made alone, none differs from
the whole game's). Version 2's weren't: a status was put on the target by the first chip of the whole game that
inflicts it, so a folder's chips made alone differed from the whole game's in their status ratios, every one of them
in most rounds, and live play had to make the whole game first (12 to 64 s for EXE5 on this Mac, once a content).

- `Fingerprinter::start(content, threads, cache)` starts its threads (`nettai_platform::thread::spawn`: the browser's
  workers too; a thread that can't start fails it, with the reason): the first reads the cache folder (the pool's
  file, `fingerprints-<game>-<hash>.json`, the same format, the chips it holds), each loads the content's runtime and a
  sandbox, and they wait for chips. `want(chips)` asks for chips (those of the game's own not made or asked for); the
  threads make them a chip at a time and give the fingerprints made out whole (an `Arc`, with a count the battle's
  thread reads without a lock), and once none is left to make, write them all to the cache file
  (`nettai_platform::fs`: in the browser, the app's kept folder). One a content in a process. `status()` says reading,
  making (chips done of those asked for) or ready, or failed (a run the engine stopped fails it too); `unkept()` why
  the cache couldn't be written. `Fingerprinter::made` takes fingerprints a host has already.
- `NetAi` asks, as each round starts, for both sides' folders' chips (the round's setup) and the Program Advances
  the content makes of them (`ai::folders_chips`). Reading the other side's folder is for this alone: only a chip of
  it shown used reaches the view. The host can ask earlier, as it knows both builds at setup (`want`), so the chips
  are made while the round opens.
- **A view describes a chip without its fingerprint** (`Obs::unprinted`, which the encoder fills where the names have
  fingerprints but not that chip's): if it is one of the game's chips, no decision is made on that view; the chip is
  asked for. In the fight the action held goes on; on the custom screen it waits and asks again the next tick. A chip
  the game doesn't fingerprint (none of its own) is a zero block, as in training. So the network never sees a chip
  whose fingerprint wasn't made, and the battle's thread never waits for one.
- **The first wait** (release, the packs, this busy Mac; both folders' chips and their Program Advances, 58 to 60
  chips, two rounds a game, nothing in the cache; the threads' runtimes and sandboxes loaded in it):

  | Game | 1 thread | 4 threads | Then, from the cache file |
  |---|---|---|---|
  | EXE6 | 1.2 s | 0.5 s | 222 KB read in 1.8 ms |
  | EXE5 | 9.1–10.0 s | 3.1–3.3 s | 218 KB, 0.7 ms |
  | EXE4 | 3.5–3.8 s | 1.2–1.3 s | 202 KB, 0.6 ms |

  The names made of them take 0.1 ms. A chip already in the cache file isn't made again, so a later session waits only
  for chips it hasn't met. The browser's Luau in wasm is likely 2 to 3 times slower (not measured). The whole game
  for comparison, on 10 threads: 1.3 s, 10.4 s and 3.4 s.

### 10.4 Threads and time

On the battle's thread, desktop and browser alike, inline. Measured (release, the packs, `examples/ai_costs`, a
network of the trainer's default widths: 1.13 M parameters; random play, 3,000 fight decisions a game):

| Game | Fight pass, median occupancy | most seen | Screen pass | Every token (194) | A view encoded | Key probe (once a screen) |
|---|---|---|---|---|---|---|
| EXE6 | 0.17 ms (68 tokens) | 0.22 ms (77) | 0.27 ms (85) | 0.76 ms | 2 µs (99th percentile 4) | 0.12 ms (most 0.57) |
| EXE5 | 0.19 ms (105) | 0.27 ms (117) | 0.27 ms (118) | 0.76 ms | 2 µs (5) | 0.58 ms (1.10) |
| EXE4 | 0.19 ms (74) | 0.27 ms (86) | 0.29 ms (86) | 0.91 ms | 2 µs (6) | 0.05 ms (0.10) |

A view is encoded once every `k` = 4 ticks, under a microsecond a tick; a fight decision every 4 ticks costs the
pass; a screen decision a view and the pass, and the key probe the first time a screen reads keys.

**The browser.** The pass built alone for wasm32 (nightly, as the web build, no SIMD) and run in node's V8 takes 3.3
to 3.5 times the native time: 0.66 to 0.85 ms a decision, 2.4 ms with every token present. With `simd128` it would
be 1.4 to 1.5 times native (0.28 to 0.38 ms). Under a millisecond every fourth frame, it stays inline on the page's
main thread; no worker is needed.

### 10.5 Weights

One file, `model/nettai-ai.safetensors`, built into the crate (`include_bytes!`), the same on desktop and web. It is
replaced only when a better one is trained. Reading it is a copy of its floats (a few ms for 1.13 M parameters);
the app reads it once and gives each `NetAi` the same `Arc<Net>` (`NetAi::of`).

### 10.6 The practice menu, with the first weights

The dummy's Action gets "Fights" back, with its Level row. They are as they were before cc24c9a58 (`git show
cc24c9a58 -- crates/nettai crates/nettai-ui docs/app.md`), with `NetAi` where `Fighter` was:

- **crates/nettai/Cargo.toml**: `nettai-training = { path = "../nettai-training", features = ["live"] }`.
- **crates/nettai/src/screens/practice.rs**:
  - `ACTIONS = [StandsStill, MovesAround, Ai, SecondPad]`; `LEVELS = [Level::Easy, Level::Normal, Level::Hard]`
    (`nettai_training::ai::Level`), `NORMAL = 1`, `kept_level`.
  - `level: usize` back in `Start`, `Kept`, `MenuChoices` (`Option<usize>`) and `Session`, with `Session::ai: Option<usize>`
    (the level plugged in).
  - `PracticeOption::Level` in `ROWS` (15) after `Action`, in `values` (`LEVELS.len()`) and `value`; `shown_rows`
    shows it under the Action while that is Fights; `lay_out_rows` puts it in or takes it out as the Action changes.
  - `dummy(in_fight)`: the custom screen the AI's (`OnCustomScreen::Ai`) for Fights.
  - `plug(live, opponent, level, seed, plugged)` as before, making
    `NetAi::of(net, LEVELS[level], seed, fingerprints)` in place of `Fighter::new(LEVELS[level], seed)`: `net`, the shipped weights
    read once (§10.5); `fingerprints`, `Fingerprinter::start(content, threads, Some(<the app's data folder>/fingerprints))`
    for the session's game, and `want` of both builds' folders' chips as the session is set up (§10.3). `start` plugs it as the session
    starts; `Practice::change_option` on the Action and the Level rows. `LivePlayer::start` calls `reset`, as the set
    starts over.
  - `NetAi::error()` or a failed fingerprint status goes to the app's log (`crate::errors::log`) and takes the AI
    out (the dummy stands still).
  - The tests back: `the_dummys_ai_is_plugged_in_while_its_choices_are_the_ais`, `the_menus_rows_have_their_values`
    (11 rows with values), `the_dummys_action_brings_its_own_row`, the visit's menu with Fights at Hard; and
    crates/nettai/src/battle.rs `the_dummy_that_fights_takes_hp_away` (with the trained weights).
- **crates/nettai-ui/ui/screens/practice.slint**: `PracticeOption.level` after `action`; its label `@tr("Level")`;
  the Action's values Stands still, Moves around, `@tr("Fights")`, A second pad; the Level's
  `@tr("level" => "Easy")`, `"Normal"`, `"Hard"`.
- **crates/nettai-ui/lang/ja/practice.po**: Fights 戦う, Level 強さ, and in context `level` Easy よわい, Normal ふつう, Hard
  つよい, with their comments.
- **docs/app.md**: the rows' table (Action with Fights; "Dummy: Level (under Fights alone) | Easy, Normal, Hard"),
  "The dummy's rows", and "The dummy that fights" written for `NetAi` (what it sees, its levels, its fingerprints'
  first wait).
- **New, a UI piece**: while the fingerprints are being made (up to 3 s for EXE5 on four threads, the first time a
  folder's chips are met), a line on the practice screen saying the dummy is getting ready, with its progress
  (`Fingerprinter::status`); its look goes through mockups first.
- The tour's steps through Fights and Level are not brought back (the user: "stop doing the tour").

  | Level | Reaction | Choice |
  |---|---|---|
  | Easy | 400 ms | samples widely, sometimes a random action |
  | Normal | 270 ms | samples from its policy |
  | Hard | 170 ms | takes its best action |

  The numbers are set by measuring the levels against each other and the scripted players. The trainer's first
  values, which the weights file carries in its `levels` metadata for the Rust side: Easy 24 ticks, temperature
  1.5, a random legal action 10% of the time; Normal 16 ticks, the policy's own sampling; Hard 10 ticks, the best
  action. `nettai_ai.evaluate --level` plays them.

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

   Checked by unit tests on the test content of all three games, and a throughput measurement on the packs. Done
   (branch learned-ai): with the bridge (step 3's module and smoke run), random builds of every setup fact, and the
   fingerprints (step 6).
2. **Trainer** (training/), alongside step 1 against the spec in §3–4: the model, PPO, the league, export, and
   evaluation calls into the pool.
3. **Bridge and smoke run.** The pyo3 module. A 30-minute local run of EXE6 that must clearly beat the masher.
4. **The app.**
   - net.rs, checked against the Python export on recorded observations;
   - `NetAi`;
   - the practice rows;
   - the wasm32 type-check.

   Done but the practice rows (branch learned-ai-app, §10): they come back with the first trained weights (§10.6).
5. **Cloud runs.** The full recipe, then the held-out-game test, then the levels' calibration. The weights go in.
6. **Chip fingerprints: in v1** (the coordinator's call, the user wanting multi-step setups learned well). Each chip
   run in a sandbox battle (§3.1): where it hits, when and how hard, what it changes and leaves, and its damage under
   each condition a setup brings; the pairs of chips that work together found from them and seeded into folders.
   Generic, kept by content hash, and a folder's chips take 1 to 4 s on one thread at battle start.

## 13. Risks

- **Forms and windows.** If it rarely learns to open the Cross window or use Beast Out, the key probe (§4.2) is the
  first lever, and a small bonus for exploring windows early in training is the second.
- **Practice settings far from training.** A full gauge every turn, for example. These are randomized in (§6.3).
- **Cost of EXE5 and EXE4 ticks.** They are three times EXE6's. Batches are balanced so EXE6 doesn't dominate.
- **Weights in git.** A few MB a time, replaced rarely.
