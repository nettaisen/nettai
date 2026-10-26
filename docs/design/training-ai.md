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
- **Kept from before:** the AI stays Rust (no Luau AI). The dummy opens the custom screen as the env did in training,
  a while after the gauge is full (since 2026-10-24; before, only the player did). The dummy plays whichever build the practice menu gives it. It picks most chips each turn.

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
github.com/nettaisen/nettai-ai   the published networks, one for each nettai minor version (§10.5), which the app
                                 downloads; none is in this repository but the test fixture's
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
| rules | 96 | 14 | field, value | own navi stats of its game's own, then own rules' state: one token a scalar that isn't at its type's default (zero, as a round's state starts: absent means default), a record's scalars one level deep, an array's elements each, lists left out (EXE6 has 43 such scalars, EXE5 79, EXE4 49; a view holds 3 to 10 tokens in EXE6, 1 to 6 in EXE5, 2 to 8 in EXE4); its source, type (bool, number, enum, reference, code, other), value (set, /100, signed log, enum index), its place in an array |
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
next decision, every 4 ticks by default (`k`, a setting; Hard may use 2), and let go when a custom screen opens:
held on, the action from before the screen came back for up to k − 1 ticks as the fight resumed, before the next
decision and unknown to it (its previous action is the screen's last choice), an A in it using the new hand's first
chip at once (fixed 2026-10-25). A press is "held, then not held", and a charge is "B held" over many decisions. The back special and charged chips fall out of these sequences. It never
presses L, R, START or SELECT in the fight. Every action is always allowed.

**Reaction delay.** The decision at tick t is made from the observation of tick t − D. Decisions come at the env's
ticks with t mod k = 0 while the fight runs (dimmed too: a counter cut-in is pressed then), so the observations a
decision can need are those of ticks s with (s + D) mod k = 0: the env encodes just those, into a ring, one encoding
every k ticks a seat. In training D is drawn from 6 to 30 ticks for each round and seat and is part of the
observation. One network then plays every reaction speed.

### 4.2 On the custom screen

The network chooses what the screen ends up holding and never presses one of its keys (the user, 2026-10-24: "why
can't you submit the whole custom screen without any cursor movement so you can avoid any issues?"). Each decision
is one choice, which the navigator carries out to its end before the next is asked:

- **A slot.** One of the 12 screen slots: a chip, OK, or a button of the rules (ChpShufl's re-deal, DustCross's
  scrap, Beast Out, EXE5's and EXE4's soul, Arm Change). A slot is allowed while the screen is choosing, if it is a
  chip it can still pick (selectable, fewer than 5 picks), a button the screen shows selectable (its cell that holds
  its state: one cell wide, or the left of two), or OK, the cursor can reach it, and the screen hasn't refused it. A
  button that counts its uses (ChpShufl's, DustCross's) turns itself off as they run out; one that counts none (the
  souls', Beast Out's) keeps 0 uses, and until 2026-10-25 a button also needed uses left: the network could never
  press EXE5's soul button or EXE6's Beast Out, and never took a Soul in EXE5 or Beast Out (EXE4's souls came
  through its offered slots). The navigator walks the cursor there along the rules' own layout
  (`Screen::target_from`), a direction held two ticks (the screen moves on the second, its auto-repeat's) and let
  go one, and presses A, held a tick and let go a tick. A pick after which the screen is the same (the picks, the
  slots' states, the phase, the form, the cursor) is refused for the rest of that screen (EXE4 refuses a chip in a
  code it doesn't come in).
- **An entry of the form list.** One of the 5 entries of the rules' form list (`Battle::form_list`, behind the
  window views `form_list_opening`, `form_list`, `form_chosen`: EXE6's Crosses), allowed while no entry is chosen,
  for an entry that holds a form, isn't marked (a Cross used this battle) and wasn't refused on this screen, where
  the key probe found the key that opens the list. The navigator walks the cursor back to where the probe was made,
  presses that key, presses a direction in the list till its cursor is on the entry (a direction that doesn't move
  it is passed over for the next), presses A, and waits while the list's own windows play out. An entry the list
  doesn't take (the key opened nothing, A chose nothing, no direction got there) is backed out of with B
  (`act::CLOSING_KEY`) and refused for the rest of the screen. The entries are in the observation as the forms
  set's list tokens (their place, marked, under the list's cursor, chosen).
- **The key probe.** At the screen's first decision, each of 7 keys is pressed once in a copy of the battle (the
  other side pressing nothing) and the env notes which ones open a window of the rules. The first that opens a form
  list, with the cursor where it is, is the navigator's way into it (`act::Opener`); the flags stay in the
  observation, so a Cross on offer is seen from the first round rather than found by luck.
- **Windows are the navigator's.** Decisions come only while the screen is choosing. A form list the navigator
  didn't mean to open, by a move on its way to a slot, is backed out of with B, and the move is left out of its
  ways for the rest of the screen: with Beast Out's button shown and the bottom row's last chips hidden, the
  shortest way from the top row to OK may begin with UP, and UP on the top row opens the Cross window while a Cross
  is on offer. The rules' other windows (Beast Out's, EXE5's soul unison, EXE4's) are animations and play out
  alone.
- **One choice at a time**, not the whole screen in one: which chips may still be picked depends on those picked
  (their codes), and the rules work that out as each pick is made. Each choice is a decision of its own, made from
  the screen as it is.
- **No delay on the screen.** A screen decision is made from the screen as it is: nothing moves on it but the
  player's own cursor, and the navigator needs it as it is.
- **A screen ends.** After 24 decisions on one screen only OK is allowed; a screen open 3,600 ticks cuts the round
  off. A cut round has no result and its value is bootstrapped, so a seat that is losing gains by holding the
  screen: whatever can stall will be learned. Twice it was (2026-10-24), each time through keys the network pressed
  itself, when the screen's actions were the slots and 7 keys: with every key allowed in a window, the first cloud
  run held EXE6's Cross window open pressing directions (its EXE6 rounds at full HP cut about 70% of the time, 15
  to 20% of training's rounds); with only A and B allowed there past the limit, the second learned within 650
  updates to press A for ever on a Cross already used (23% of its EXE6 rounds cut). Now every choice is a bounded
  run of presses that changes the screen or is refused, and nothing the network chooses holds a screen open.
  Tested: the navigator reaches every slot it may point at, and chooses every entry of EXE6's form list from the
  screen as it opens and with the cursor moved away by a pick first; an entry the list doesn't take is refused and
  masked; seats choosing at random over 234 EXE6 rounds (259 Crosses chosen) have every screen closed, the longest
  in 280 ticks, and no round cut at the screen.

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
- with chance `randomize` (default 0.75), what the app's build creator lets a player change besides drawn, and only
  that, everything else as the creator leaves it (the user's standing rule: everyone at max power; a setup never offers
  HP, Regular memory, levels or sun). The creator's tabs edit the engine's roles (the navi, the folder, its Regular and
  tag chips), the facts of forms, the choices (by their presets), and its lists (a board's pieces and size, the patch
  cards, the auto battle places, the party); every other fact keeps the new build's value, and `Builds::as_built`
  puts those no build states (`never_stated`) back at the rules' defaults. So the filler draws the lists a build
  states (not a list of other navis: a side's party, whose switching the env doesn't play, out of scope for now) and
  no single fact: a board's size stays the new build's, the largest. An entry's fields are drawn from what their types
  allow (a flag, a variant, a definition of the game's, a number below 8: the creator's own ranges, a board's size,
  are its screens' code, not data a crate below the app reads), and an entry is kept only where the rules'
  `validate` and `build_check` say nothing of it, asked of the side's round set up with its setup block changed in
  place. EXE4's builds data didn't name `sp_times` among the facts no build states (EXE5's and EXE6's do; no tab
  offers it): it does now. Before, the filler drew single facts too: a board's size of none left MegaMan with no
  NaviCust and his fresh 100 HP (max HP from 1 in EXE6, 40 in EXE5; 3.5% and 2.5% of builds under 100), `beast_out`
  locked, random `bug_frags`. Now, over 60 builds a game, every seat's max HP is the creator's overview's (a test
  checks it, and that every single fact is a new build's): EXE6 800 to 2,000 (median 1,320: HP+ programs up, a card's or
  a bug's HP down), EXE5 600 to 1,400 (its team navis at their last level's), EXE4 997 to 2,700, none under 500;
  Those lists were sparse, though (the user, of the HP gap the duplicate deals showed, 2026-10-24: "megaman's hp can
  change through navicust though", "there's all kinds of navicust programs", "sometimes bugs on the navicust are
  advantageous", "you might want to take patch cards into account as well"): an entry drawn whole is rarely one the
  rules take (a color its program doesn't come in, a turn past 3, a place off the board), so ten refusals in a row
  ended most lists early. Over 80 randomized builds a game, NaviCust programs averaged 0.2 (none in 69 of 80, EXE6),
  0.1 and 0.3; patch cards 3.1 of 32, 2.7 and 1.2 of 7; auto battle places under 1 of 42. Now half the randomized builds
  (`builds::FULL_SHARE`) fill each list full: to its room, refused 60 times in a row at most (`FULL_PATIENCE`), its
  entries mostly from the list's pool (`Maker::pools`: those of 4,000 drawn on a seed of the list's that the rules
  take alone in a new build of the navi that changes form, made once a maker, the same whatever thread asks first),
  a field drawn anew in half of those, the rest drawn whole; nothing the rules take is judged (bugs come as the places
  make them, every kind of program and card as drawn). Full: NaviCust programs 5.5 (EXE6; ~9 a MegaMan build), 4.1
  and 9.0; patch cards 4.1, 4.0 and 5.7; auto battle places ~39; EXE4's max HP mean 1,216 → 2,118; 38, 85 and 128 ms a
  build (8 to 10 ms the others), the pools 0.6 to 1 s (`how_full_randomized_lists_are` prints these);
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

A round between two builds is a match of one round on a random arena of the game's, each side's HP as its build has
it (the creator's). With the pool's `hp` option set, both sides' base HP is the drawn HP where the rules take one
(`PlayerFact::BaseHp`); a navi whose HP is its level's (a link navi) keeps its own, as the rules refuse a base HP for
it.

### 6.3 Who opens the custom screen

Neither side learns to open it. The env opens it for one side or the other a random while after the gauge fills
(`env::opener_wait`): 0 to 10 seconds, weighted toward soon (600 u² ticks, u uniform), pressing L (a press and a
let-go until the screen opens). No seat's own L or R reaches the fight. The app's player (§10.2) opens its own the
same way (the user: "allow the dummy to go into custom screen"); the player may open it first.

The practice options that change the battle are on in some rounds, each with chance `practice` (default 0.25), since
the dummy meets them: a full gauge (whenever it runs, it is full), and the folders dealt in their own order. Both are
nettai-frontend's training session's (`Training`), done in the env on the battle's state as the session does them
(the frontend is out of reach without the `live` feature).

### 6.4 Rewards

- **The round's result:** +1 for a win, −1 for a loss, 0 for a draw (`reward.result` × it), by a KO.
- **A stall costs** (the user, 2026-10-24: "you should penalize winning by stalling more because that's boring").
  Link rounds end on the judge's ruling on damage once the final turn's timer runs out (turn 15): a seat that takes
  a lead and runs the clock out wins there. A seat that didn't lose a round decided by the judge gives up
  `reward.judged_cost` (0.75): a judged win is worth 0.25, a judged draw −0.75, a judged loss −1 as a KO's, so the
  leader gains by finishing and the one behind gains nothing by running. A round that reaches the fight's limit
  (`max_fight_ticks`, 5 minutes of fight) ends there, worth `reward.limit` (−1) to each seat: before, it was cut
  off and bootstrapped from the value, so whoever was ahead kept the lead's worth by stalling to it. The pool says
  how each round ended (`ending`). Two players that never attack go to the judge after 4,300 ticks of fight (a
  test).
- **A cut** (a screen open too long, the engine stopping the round) is 0 for both, with `truncated` set, and
  bootstrapped.
- **HP difference as it happens**, weighted down over training: (HP the other lost / the other's max HP − HP it lost
  / its own max HP), so a whole KO adds up to 1 whatever the HP (the coordinator's change from / 1000). A round's end
  credits the ended round's alone: what a seat gains in the next round before its first decision is no transition's.

Nothing else for behavior. Rewards for particular behaviors ("use chips", "don't pace") get exploited; strength comes from
winning.

### 6.5 The pool

- `Pool` runs N envs on persistent worker threads. A thread keeps its Luau runtimes (a game's takes about 1 s to
  load) for its whole life, and any worker steps any env (a battle is plain data).
- `step(actions)` hands every worker the batch's actions. Each claims the next env not yet stepped, steps it to its
  next decision and writes its rows of the buffers, until none is left: a slow core (an efficiency core), a round's
  start or a key probe holds up no other worker's envs. The buffers are the pool's own memory, never reallocated, and
  the bridge shows them to Python as numpy arrays with no copy.
- Each env has both sides' seats; Python says which policy plays each seat.
- Builds are made ahead, two a game an env (`READY`), by the pool's `builders` threads (the trainer's
  `env.builders`, 4), or with none by the workers while they wait for the next step (Python's forward pass). Each
  game has its own stream of draws for its builds, used in order, one build at a time, so a round's builds are the
  same however busy the threads were: a pool of one thread, one of three and one of three with two builders play the
  same, buffer for buffer (tested, with every randomized build's lists full). A build is made with its game's stream
  taken out of the env's build queue (`env::EnvBuilds`), the env's lock not held, and a round that needs one being
  made waits for it. (Until 2026-10-24 a worker made one holding the env's lock and answered no step until it was
  made: with full lists, §6.2, builds of up to 130 ms against steps of a millisecond or two, the fourth cloud run fell
  from ~27,000 decisions a second to ~16,000. On the Mac, 96 envs of the three games, 8 workers, random actions, after
  the queues filled: lists never full 34,500; half full, the workers making builds 24,700; with two builders
  33,800.)

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
    hp=None,                         # (lo, hi): both sides' base HP drawn per round, where the navi takes one;
                                     # None: each side's as its build has it (the creator's)
    chip_holdout=(0.0, 0, "exclude"),  # (fraction, seed, mode): chips held out of training, below
    practice=0.25,                   # the chance each practice option is on in a round
    randomize=0.75,                  # the chance a build draws every fact it states (§6.2)
    combos=0.3,                      # the chance a random folder is seeded with a pair of chips (§3.1, §6.2)
    cache=None,                      # a directory the chips' fingerprints are kept in, by content hash
)
spec = pool.spec()
# {"hash": "<hex>", "buckets": 8192, "fight_actions": 20, "screen_actions": 17,   # 12 slots, then the form
#  "form_entries": 5,                                                           # list's 5 entries (§4.2)
#  "probed_keys": ["up", "down", "left", "right", "a", "b", "start"],
#  "chip_tokens": {"hand": (0, 6), "used": (6, 1)},                              # (first, count) in "chips"
#  "fingerprint": 114, "fingerprint_features": [(name, width), ...], "games": [...],
#  "k": 4, "screen_decisions": 24,                                              # what the envs use (not hashed)
#  "sets": {name: {"count": C, "width": W, "ids": I, "id_names": [...],         # sets in a fixed order:
#                  "features": [(name, width), ...]}, ...}}                     # general, panels, objects,
#                                                                               # chips, slots, rules, forms
buf = pool.buffers()   # numpy views of memory the pool owns, refilled in place by every step(): the same arrays
# per set:  buf["x"][name]    float32 [E, 2, C, W]
#           buf["ids"][name]  int32   [E, 2, C, I]   two hashed ids a name, 0 = none
#           buf["mask"][name] bool    [E, 2, C]      true = token present
# buf["phase"]        int8    [E, 2]   0 no decision this step, 1 fight, 2 custom screen
# buf["legal_fight"]  bool    [E, 2, 20]
# buf["legal_screen"] bool    [E, 2, 17]  the slots set's 12 tokens, then the form list's 5 entries
# buf["hp_gain"]      float32 [E, 2]   HP the other lost / its max HP - HP it lost / its own, everything since this
#                                      seat's last decision (steps where it had phase 0 too); read where the seat
#                                      decides or the round ends
# buf["done"]         bool    [E]      the round ended; the obs is already the next round's first
# buf["truncated"]    bool    [E]      cut off (a screen open too long, the engine stopped)
# buf["ending"]       int8    [E]      how a round not cut off ended: 0 a KO, 1 the judge's ruling, 2 the fight's
#                                      limit (max_fight_ticks, no result) (valid where done)
# buf["result"]       int8    [E, 2]   +1 / -1 / 0 for the round that ended (valid where done)
# buf["game"]         int8    [E]      index into games
# buf["delay"]        int16   [E, 2]
pool.step(actions)                    # int32 [E, 2]; read only where phase != 0; a step with an action not
                                      # allowed is refused whole (an exception), nothing moved
pool.set_seat(envs, seat, controller) # "policy", "still", "mover", "masher", "shooter", "dodger"
pool.set_delay(envs, seat, ticks)     # pins the delay (evaluation); None draws again
pool.set_k(envs, seat, k)             # the seat's own k (evaluation: each network at the k it trained at);
                                      # None: the env's
pool.set_deal(envs, key)              # envs dealt the same rounds from key (game, builds, HP, arena, seed, drawn
                                      # delays, practice options: from the key and the round's number since);
                                      # None: each its own draws
pool.set_hp(lo, hi)                   # set_hp(None): each side's as its build has it
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

- The first round begins when the pool is made, every seat "policy". `set_seat`, `set_delay`, `set_k`, `set_deal`,
  `set_hp` and `set_chip_holdout` take effect from the next round (the one after the round each env is playing).
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

As built in `training/nettai_ai/model.py`. `training/MODEL.md` gives the exact computation the Rust port
follows: tensor names and shapes, op order, the LayerNorm's eps, the GRU's gates, how masks apply.

- **Tokens.** Each token set has its own small MLP (its floats, normalized by running statistics and clipped at
  ±5, to 32, ReLU, to d), plus a learned vector for the set, plus its names. Every hashed name comes as **two
  ids**, from two seeds of the same hash (the coordinator, 2026-10-23); both are looked up in one table shared by
  every set (8,192 × d) and summed in. Id 0 is no name. Chip tokens carry a **fingerprint block** of fixed width
  in the spec, zeros until §12's step 6 fills it; nothing reads it specially.
- **Mixing.** One pre-LayerNorm transformer layer (4 heads, feed-forward 2 × d, ReLU) over the present tokens of
  every set but the **pool-only** ones (`model.pool_only`, the rules' scalars: 43 tokens every decision in EXE6,
  79 in EXE5, 49 in EXE4, where §3's table expected up to 24). Their relations to the field matter little, and
  in the transformer they would be most of its cost; the summary's pool reads them. Absent tokens are skipped,
  not computed and masked.
- **The chips set's two parts.** The encoder sends the hand and the other's used chip as one `chips` set of 7
  tokens and says where each part is (`spec()["chip_tokens"]`: hand 0–5, used 6). The trainer reads them as two
  sets, `hand` and `used`, in `chips`' place (each its own MLP, and the used chip a set of one, read directly
  below); the weights' architecture records the split for the Rust side. The pointer's set is the encoder's
  `slots`.
- **Summary.** An attention pool (4 learned scores over the tokens, a weighted sum each), plus the token of
  every set of one (`general`, the other side's `used` chip) read directly, to 128. Reading those directly was
  the difference on FakePool's toy between learning the fight (its target is in the general token) in 40
  updates, 71% of targets pressed, and not, 19%: a pool over 66 tokens starts out reading the one that matters
  at 1/66.
- **Memory.** A GRU of 256, its input the summary and the stream's **previous decision's action**, one-hot: the
  fight's 20 actions, the custom screen's 17 (12 slots, the form list's 5 entries), and none (a round's first
  decision, where the GRU's state is zero too): 38 values after the summary's 128 (the architecture's `previous_action`). The previous
  decision may be of the other phase (a fight action before the custom screen opened). It decides from a view D
  ticks old (D 6 to 30 in training, up to 7 decisions back), so without it the network can't know what it pressed
  since: whether it already pressed A (and wastes a chip pressing again), already stepped (and overshoots), or is
  charging. Feeding the recurrent core its last action is the standard answer (IMPALA, R2D2, the delayed-MDP
  literature), and fair under §0's rule: a player knows its own presses. Concatenated rather than embedded and
  added: the one-hot's product with the GRU's input weights is a column of them, so the Rust pass adds one column
  (no multiply), and a warm start (§8) is zero columns. Why it came: §11's stall.
- **Heads**, on the GRU's state and the summary:
  - the fight's 20 actions;
  - the custom screen's pointer: a score for each slot token (an MLP over the slot's token and the rest), and a
    logit for each of the form list's 5 entries (`entry1`, `entry2`), 17 in all;
  - a value.

  Legal masks are −inf before the softmax; an absent slot is −inf too.
- **Name dropout** (the coordinator, 2026-10-23). In training, each of the learner's tokens loses its name ids
  with probability 0.2 (a setting), in the rollout and, as stored, in the update; never in evaluation, export or
  play. A chip or object kind the network has never seen is then not noise: it has learned to play from the
  numbers beside the name.
- **Budget, as decided.** This section's first draft (width 128, two layers, ~140 tokens) was about 30 million
  multiply-adds a layer, ten times the budget. The coordinator's rule (2026-10-23): count present tokens only
  (the Rust port skips absent ones), at a typical fight position and at the max; targets 3 M typical, 10 M max.
  - The typical position, as the pool shows it (random play, 2026-10-23): general 1, 18 panels (the attack log is
    in their tokens), about 10 objects (4 to 5 in random play, up to 17), 6 in hand, the used chip, no slots, 57
    rules scalars (the three games' mean), 2 forms: 95 tokens, 38 of them in the transformer.
  - The default, **d = 64**, one layer, on spec c4d9e4a61b0f54ba (chip tokens 250 wide with their fingerprints):
    2.22 M typical, 2.95 M on the custom screen (its 12 slots added), 5.82 M max (194 tokens); 1.17 M parameters,
    0.52 M of them the name table; 4.4 MiB (1.13 M and 4.3 MiB before the previous action's columns). (Before the rules were made pool-only, d had to be 56 at 66 tokens;
    with all 95 in the transformer it would be over 4 M.)
  - `--size large` (d 128, two layers: 13.6 M typical, 36.5 M max, 2.6 M parameters) is there to try in the
    cloud. If it plays clearly better, the app runs it on a worker thread (§10).
  - `uv run python -m nettai_ai.model [--size large]` prints the numbers; training logs the cost at the
    measured occupancy (`model/macs_measured_fight`).

## 8. Training

As built in `training/nettai_ai/` (ppo.py, rollout.py, league.py, curriculum.py, device.py, train.py).

- **PPO.** Clipped at 0.2 (the value too), GAE λ 0.95 and γ 0.997 per decision, about 20 seconds of horizon at
  4 ticks a decision. Entropy bonus 0.01, gradient norm 0.5, advantages normalized per minibatch, 3 epochs, Adam
  at 3e-4 falling linearly to a tenth. It uses the GPU for updates and batched inference, and Rust for
  everything else.
- **Tried and dropped (2026-10-25, commit 637dd6959, reverted).** Crediting each hit to the decision whose own
  action started it (within 120 ticks), the fight decisions' GAE at λ 0.8 with their advantages normalized apart from
  the screen's, the last 16 decisions of a round held for credits to come, and auxiliary heads (the other loses HP
  soon, the seat does, the two share a row). From the fourth run's update 3,750, 400 updates on the Mac each way:
  against the still bot (EXE4, no delay, HP only) the kill speed was 521 HP/min against the old way's 579; against
  the masher, shooter, dodger and mover (delays 6 to 30) the two played even over duplicate deals (72 deals won both
  ways to 76; Elo 1,316 to 1,322), with the same hits taken. The diagnosis that led to it (the network "doesn't
  aim") was wrong: the scripted shooter, which walks to the other's row and fires at once, kills a still navi slower
  (133 s a round in EXE4) than the network (122 s), and beats the masher only half the time (50 to 52%, against the
  network's 68 to 80%); about half of all chips can't hit a still navi at all. What the numbers do show is little
  dodging: the network takes 95, 83 and 63% (EXE4, 5, 6) of the masher's damage a still navi takes.
- **Each seat is its own stream.** A pool step brings decisions for some seats (phase ≠ 0) and none for others.
  Each (env, seat) the learner plays is its own trajectory of decisions, with its own GRU state, reset when a
  round starts. GAE runs over each stream's own decisions, never over pool steps. The trainer reads §6.6 so:
  - a decision's reward is read at the seat's next decision, or at its round's end, whichever comes first:
    `hp_gain` there, times the HP weight, plus, at a round's end, `result` times the result weight;
  - so `hp_gain` must cover everything since the seat's last decision, including steps where the seat had no
    decision (phase 0); the trainer doesn't read it on those steps.
- **Cut-off rounds.** A round `truncated` (18,000 ticks of fight, a screen open 3,600 ticks, or the engine
  stopping it: about 1 in 50 EXE5 rounds ends on a content error today) has no result. The trainer bootstraps it
  (below) and counts it for nothing: not in the league's win rates or Elo, the curriculum, or the behavior
  measures; evaluation counts it apart. `uv sync` drops the bridge from the trainer's environment (it isn't in the
  lock file): after building it, run with `uv run --no-sync`.
- **Rewards' weights.** The result 1; the HP difference from 1 down to 0.2 over the first 3,000 updates.
  `hp_gain` is a fraction of the round's max HP (the coordinator, 2026-10-23), so the shaping for a KO is the
  same at any HP of the curriculum.
- **A minibatch in parts** (`ppo.accumulate`, default 1). A network whose minibatch doesn't fit the GPU's memory in
  one pass (the extra-large run's on a 32 GB card at 2,048 envs and 64 chunks) takes it in that many parts, each
  adding its share of the minibatch's mean losses (the advantages normalized over the whole minibatch) before the one
  clipped step: the same step as one pass (a test, in 1, 2 and 3 parts).
- **The rollout's slices and their bootstrap.** A rollout is 128 pool steps. A stream's last decision in it is
  still open: its reward and the next state's value come later. It is carried into the next rollout as that
  rollout's first decision and trained once it closes; the decision before it is bootstrapped from its value. A
  round cut off at `max_fight_ticks` (truncated) is bootstrapped from its last decision's own value, since the
  pool's observation is already the next round's; a KO or a judged time-up is terminal.
- **Truncated BPTT.** Chunks of 128 decisions (`ppo.chunk`), each started from the GRU state stored when its
  first decision was made, and zeroed again at each round's first decision inside it.
- **The previous action** (§7). The collector keeps each stream's previous decision's action for every stream a
  network plays (the learner's and each snapshot's), set to none with the GRU state as a round starts, and gives
  it to the stream's network with each decision. The rollout stores it with the decision (an open decision
  carries its own into the next rollout), and the update reads it from there, never derived from the stored
  actions, so it replays exactly what the behavior network read: a test checks every stored one is the stream's
  last action (none at a round's first) and replays each decision's log-probability. Evaluation keeps its own
  the same way.
- **League.**
  - About 50% of rounds against its latest self (both seats learn).
  - About 35% against frozen past versions. A snapshot joins every 10 updates. New snapshot rounds draw from
    2 active snapshots, picked by weight (1 − the learner's win rate against it)², the rate over its last 200
    rounds with a win and a loss of prior: those that still beat it are picked most. Until the first
    snapshot, this share is self-play.
  - A round keeps its opponent to its end, about eight rollouts, and each snapshot in play costs the
    collector one network pass a step (about 1.1 s a rollout on the cloud machine). So the number in play is
    bounded. Before each rollout the longest-serving active slot turns over to a new draw, but only while
    fewer than 2 dropped snapshots still play out their rounds: at most 4 snapshots play at once. The first
    cloud run drew 4 new active snapshots every rollout. Rounds kept older ones, so 9 played by update 94,
    past the cache of 8 loaded networks, and collection went from 14 s to 56 s a rollout.
  - On disk: past 200 snapshots, the file of the one the learner beats most is deleted (never an active or
    in-play one; its record stays). The first cloud machine's disk is 16 GB.
  - About 15% against the scripted players, evenly.
  - The learner's seat is drawn each round, so the seats mirror.
  - Online Elo (K 16) for the learner, the snapshots and the scripted players, logged.
  - **Exploiters** (2026-10-26, the user: "do 2 and 3", after the large network's gains slowed to about 25 Elo
    a 500 updates): another run, warm-started from the main one, whose rounds are all against the main run's
    latest network (`league.external` 1, its `external_dir` holding a copy of the main's checkpoint, refreshed
    every half hour), so it learns what beats the main network as it is; and the main run plays the exploiter's
    snapshots in `league.external` of its rounds (0.15, taken from self-play and its own snapshots), picked by
    PFSP as its own are, so it must answer what the exploiter finds. External opponents are another run's weight
    files (a snapshot's state dict or a checkpoint), the directory rescanned each rollout and a changed file read
    again (`League._scan_externals`), and bounded in play as snapshots are: new rounds draw from `active` of
    them, at most `draining` dropped ones still playing (the cache holds both sets). Unbounded, the main run drew
    from all of the exploiter's snapshots at once: 28 within 40 minutes, more networks than the cache held, read
    from disk again and again, and its rollouts took 200 s instead of 35. Both runs share the main run's machine
    (each update's minibatch in parts, `ppo.accumulate`, to fit the GPU); the exploiter keeps a snapshot every 50
    updates. **It found nothing:** warm-started from the large run's update 2,450, in 2,050 updates (about 66 M
    decisions, five hours) its win rate against the large run's latest network stayed between 42 and 55% (50% at
    the end), and the large run's against its snapshots at 50%; the large run's update 2,750 against its 2,450 (the
    last before the exploiter), over duplicate deals at Hard, won 69 deals both ways to 56 (Elo 1,183 to 1,177),
    within the noise, as its 2,450 had been even with its 2,000 (65 to 61). So it was stopped (2026-10-26), the
    large run back on its own league shares with its learning rate cut (`ppo.lr` 1.15e-4 from update 2,850: 1.0e-4
    there, falling as before, where the schedule had it at 2.6e-4).
- **Plans a round ahead.** The pool takes a seat's controller "from the next round", so each env's next round
  (the learner's seat; self-play, a snapshot or which scripted player) is sent with `set_seat` as the current
  one starts. Which snapshot plays a policy seat is the trainer's own choice, made when the round starts. The
  trainer takes the pool's first round, begun when the pool is made, as self-play: every seat starts as
  "policy".
- **HP curriculum** (the coordinator, 2026-10-23). Both sides' max HP starts at 250 and rises by 125 to 1000.
  A round draws its HP from (level − 250, level). The level rises when the learner wins 75% of at least 100
  rounds against the masher at that level, or after 500 updates at one level. The last quarter of training is
  at 1000 only. The pool takes `hp=(lo, hi)` and `set_hp(lo, hi)`, from the next round.
- **Input normalization.** Per set and feature, a running mean and variance over present tokens, merged after
  each update (so the behavior network, one update behind, has the statistics of one rollout fewer; the PPO
  ratio counts that as part of its policy's change); set first by 32 pool steps of the untrained policy. It's
  part of the weights, so each snapshot keeps its own.
- **The loop: collection and the update overlap** (the user's choice, 2026-10-23; train.py). It is the only way
  training runs; there is no synchronous mode beside it.
  - **Two threads.** A collector thread owns the pool, the league, the curriculum and the **behavior network**,
    a copy of the learner's weights. It steps the pool (the bridge lets go of the GIL in `step`) and runs every
    network's inference for the step: one forward pass a network (the behavior network for the learner's seats,
    each active snapshot for its own), all seats needing a decision batched. The learner, the calling thread,
    runs GAE and the PPO update.
  - **Two rollouts, handed back and forth.** The collector hands a full rollout to the learner and takes the
    other one, which the learner hands back after its update with a copy of its new weights. The collector
    loads them as its behavior network before playing the next rollout. So rollout k is always played by the
    weights after update k − 1: the policy lag is one update (the first rollout of a run is the learner's own),
    whatever the threads' timing, and a run is the same from run to run (tested). An open decision at a
    rollout's end is carried into the other rollout as before, played by its own version.
  - **PPO against the behavior policy.** The loss's ratio is against the log-probabilities the behavior network
    stored with each decision, the value clip around its values, and each chunk starts from the behavior
    network's GRU state stored with its first decision (a test replays every stored decision with the version
    that played it). The log shows the lag (`loop/policy_lag`) and the first epoch's mean ratio and KL against
    the behavior policy (`ppo/behavior_ratio`, `ppo/behavior_kl`).
  - **Snapshots and the curriculum** are the collector's: a snapshot is the behavior network as it takes up
    every `snapshot_every`-th version, and a rollout's rounds move the curriculum before the next rollout, as
    after each update before.
  - **Compact rollouts.** A decision keeps only its present tokens, in order, each with its position (the
    network reads no position but the pointer set's, which is laid out by position again when read): about 34
    tokens of 194, 9 KB of 57. Tokens and GRU states are on the device; the rest of a decision (legal masks,
    action, log-probability, value, reward, where its tokens are) on the CPU, where the collector writes it
    without waiting on the device, and the update copies what it reads. Each step's deciding streams are
    gathered from the pool's buffers on the CPU, present tokens only, into pinned memory and copied without
    waiting; a step waits on the device once, for its actions (with their log-probabilities, checked finite
    there). On CUDA each thread has its own stream (the collector's at the higher priority: the pool waits on
    it), and every handoff carries an event the other side waits for.
  - **Evaluation** runs on a thread and pools of its own, with a copy of the learner's weights at the update it
    started at; its results join that update's row when they come. One runs at a time. This is simpler than
    fitting its steps into the collector's waits, and its pools (2 threads in `configs/cloud.toml`) are small
    beside the training pool. **Checkpoints** are written by the learner after its update, from the league's and curriculum's states
    the collector handed over with the rollout (a few hundred ms every 50 updates; the collector goes on).
  - **Threads.** torch's CPU threads are set per thread (`run.torch_threads` for the collector,
    `run.update_threads` for the learner); they run at once beside the pool's own.
- **Games.** Every loaded game, mixed in each batch.
- **Logs and checkpoints.** TensorBoard (viewed over an ssh tunnel): rewards, losses, KL, explained variance,
  win rates by opponent, Elo, §9's measures from `round_stats()` by opponent kind, the HP level, the measured
  network cost. A checkpoint every 50 updates (a setting), with the run's settings, the git commit (a run
  outside a git checkout fails), the observation spec, the league and the optimizer; `--resume` continues it.
- **Warm start** (`--init-from CHECKPOINT`, a file or a run directory's latest). A new run (update 0, a fresh
  optimizer, league and curriculum; the rest of the config a new run's) whose network is the checkpoint's, of its
  model widths (`model.warm_start`): every tensor of the same shape copied, the input normalization with its
  running statistics, and the GRU's previous-action columns zero where the checkpoint's network read none (the
  first cloud run's), so it plays as that network did until training moves them; no normalization warmup. A
  checkpoint from before the form list's entries (its screen's actions after the slots 7 keys, heads `key1` and
  `key2`: the first two cloud runs') loses those heads, the entries' head starting as a new network's, and of the
  GRU's previous-action columns keeps the fight's, the slots' and none's, the keys' left out and the entries'
  zero: its fight, its slots and its value are the checkpoint's (tested on the toy's tensors). A checkpoint of
  another spec is converted to the pool's first (`uv run python -m nettai_ai.convert CHECKPOINT OUT`: its
  network warm-started on the pool's spec, its observation required to be the pool's, its recorded decisions with
  the new layout's allowed actions; the optimizer and league left out, so it can't be resumed), and evaluation,
  export and `--init-from` take it as one of their own. Its
  checkpoints keep where it came from (`init_from`: the path, update, decisions and commit). Tested: the same
  outputs as the checkpoint's network whatever the previous action, on the toy's tensors and (once, by hand) on
  the first run's checkpoint 1,000 against the old code: 3.4e-7 at most over its 16 recorded decisions. A smoke run
  from checkpoint 1,500 on the real pool (three games, 48 envs): the first epoch's ratio against the behavior
  policy 0.999, explained variance 0.65 from the first update.
- **Export.** `nettai_ai.export` writes safetensors with the architecture and the **observation spec hash** in
  its header. The Rust side refuses weights whose hash doesn't match its own encoder, so a changed observation
  can't silently play an old network. Beside it, a parity fixture: a seat's recorded decisions from training and
  the network's outputs for them.
- **Checked on FakePool**, a numpy stand-in for the pool with the pool's sets and widths and a learnable toy
  (press the target action shown in the general token; on the screen, point at the slot with the highest first
  float; everything else noise): in 40 updates on a laptop CPU, targets pressed 7% → 35%, best slots 10% → 29%,
  reward per decision ×7 (the test, with the loop overlapped; taking turns, before it, 48% and 38%). By 2026-10-24
  the test's seed came to 32% (24 to 33% over seeds 1 to 4), and with the previous action (§7), which moves every
  random draw after the GRU's, 28% (24 to 30%): seed noise, its bar now three times the random policy's, 21%. Over three
  seeds of 60 updates the lag costs little: targets 55% against 58% taking turns, best slots 37% against 37%;
  at 60 s of wall time on the Mac's CPU, 39% and 35% against 40% and 28%.
- **And on ComboPool** (`fake.py`, the previous action's toy): a fight seat's decisions alternate between an
  opener, of which only one of 4 is allowed, drawn at random (the network reads no legal mask, and nothing it
  sees says which), and its finisher, every action allowed, the opener's own finisher hitting; the view shows the
  opener two decisions late. Rates of finishers right over 5 updates, of 100: with the previous action 0.06, 0.11,
  0.23, 0.46, 0.82, 0.94 at 50, 0.97 at 100; with it hidden (always none), 0.25 from about update 45 to the end:
  one of the 4 finishers, as the toy allows. The test runs 60 updates of each (0.83 and 0.22 as committed).

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
  - forms taken: screens left with a form chosen, and forms entered however they came to (a Cross, a Soul, Beast
    Out: the navi out of its game's base form) and the share of the fight spent in one.
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
  game". It is run once the main recipe works: `train --train-games exe5,exe6 --eval-games exe4` evaluates on
  EXE4 every `eval.every` updates.
- **Chips it never trained on.** `env.chip_holdout = 0.1` keeps a tenth of each game's chips (drawn by a seed)
  out of training builds and favors them in evaluation builds. The pool takes it as
  `chip_holdout=(fraction, seed, "exclude" | "favor")`.
- **The driver.** `nettai_ai.evaluate` plays a checkpoint against each scripted player in each game (a pool per
  game) and against other checkpoints, the envs split evenly over the opponents and both seats, each network
  deciding at the k of its training (`set_k`; the fourth cloud run's is 8, the earlier runs' 4). Each env's
  first round is dropped, since seats, delays and k change from the next round. `--duplicate` plays each deal
  twice with the seats swapped (`pool.set_deal`: an env and one of the same pairing the other way round share a
  key) and counts the deals each player won both times, lost both times, or split (the deal decided: 50% if the
  deal didn't matter between even players, 100% if it decided every round); `--mirror` gives both sides side 0's
  build (`env.mirror`), so with `--duplicate` neither the builds nor the chip draws decide, play alone (what a
  player who beats a scripted one every time beats it at; the large run's update 2,000, mirrored, won 75, 70
  and 86% of rounds against the masher in EXE4, EXE5 and EXE6, 79, 76 and 83% against the shooter, 97–99%
  against a player who does nothing: about what it wins with two random builds, so its losses are its play,
  not the builds); `--records` writes every counted
  round's record (JSON lines). It prints a table (wins, draws,
  losses, and every numeric measure of `round_stats()` per minute) and Elo by Bradley–Terry over every round
  (a small prior keeps a perfect record finite; the scripted players' mean is 1000). From `round_stats()` it
  reads `env`, `ticks` and `seats` (two dicts, each with `result` and any numeric measures).
- **The user plays it.** The decisive test.

## 10. In the app

As built (branch learned-ai-app): the forward pass (`net.rs`) and the player (`ai.rs`, feature `live`, the only
place nettai-frontend is used) in nettai-training. The practice menu's rows came back with the first trained
weights (§10.6, branch learned-ai-ui).

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
  the GRU (gates r, z, n) over the summary and the previous action (`Memory::last`, the column of its one-hot's
  place added to the input's product; the weights' `previous_action` must be this pass's 38); the heads of
  `core = [h', z]`. Absent tokens are never computed. A decision computes the
  heads it reads (`Heads::Fight`; `Heads::Screen`, the form list's entries and the pointer, whose core part is computed once;
  `Heads::All` adds the value, for the parity test). A player's `Memory` holds the GRU state, the last decision's
  action (`Memory::took`: the player says what it chose) and room for a pass, so a decision allocates nothing;
  `Memory::reset` zeros the state and forgets the action.
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
  GRU carried from a random h0 and reset at each game, each decision's previous action its stream's (fight and
  screen actions, none at each game's start). Every finite output matches within 1e-4 and every −∞ exactly:
  the raw and masked logits, the value and the GRU state after each decision. The trainer's default widths (d 64,
  four heads, one layer) were checked the same way once (`--size default`), not committed: its name table alone is 2
  MB. Weights of another spec, format version, an unknown architecture field, a previous action of another width
  or a missing tensor are refused (tested).

### 10.2 The player

`NetAi::new(weights, level, seed, fingerprints)` (the weights as bytes, so the app can `include_bytes!` the shipped
file; `NetAi::of` takes a network already read, for an app that keeps one); `Level` is Easy, Normal or Hard, its
knobs the weights' by name. It plays a side as the env plays a policy seat (§4, §6), once a tick on the battle's
thread (`fight` or `custom_screen`, both the same tick of the round):

- It opens the custom screen as the env does (§6.3): a while after the gauge is full, drawn anew each time
  (`env::opener_wait`, its own draws), L pressed and let go until the screen opens.
- It counts ticks from the round's start, keeps the attack log, and encodes the view into its reaction-delay ring
  on the ticks a decision will need (`(t + D) mod k = 0`, `D` the level's delay).
- In the fight it decides every `k` ticks from the view of `D` ticks before (what it sees now while the round is
  younger than that) and holds the action till the next decision.
- On the custom screen it decides whenever its navigator is idle and the screen is choosing, from the screen as it
  is, the keys probed in a copy of the battle at each screen's first decision, only OK after 24 decisions, the
  refused slots and entries masked; each new screen's navigator starts from nothing. All in the env's order.
- `k` and the 24 decisions are the weights' metadata (`k`, `screen_decisions`), the env's in training: a weights
  file without them is refused.
- It never reads `theirs`: it doesn't hear the player's buttons (tested: the same play whatever it is told).
- The GRU state carries through a round and starts over at the next (a new round is a new score of the set);
  `reset`, as the set starts over, starts everything over, its draws too (tested: the same buttons again). Its
  last decision's action goes with it: each decision tells the network's memory what it took (the fight action it
  holds, the slot or key it points at), which the next decision reads, and a new round or `reset` forgets it with
  the GRU state (tested). A decision that waits for a fingerprint takes nothing, as none is made.
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
pass; a screen decision a view and the pass, and the key probe at a screen's first decision.

**The browser.** The pass built alone for wasm32 (nightly, as the web build, no SIMD) and run in node's V8 takes 3.3
to 3.5 times the native time: 0.66 to 0.85 ms a decision, 2.4 ms with every token present. With `simd128` it would
be 1.4 to 1.5 times native (0.28 to 0.38 ms). Under a millisecond every fourth frame, it stays inline on the page's
main thread; no worker is needed.

### 10.5 Weights

**Published apart from the app** (2026-10-26, the user: "pull the model out into its own repo so the model can be
updated by nettai without having to pull a new release of nettai, unless the minor version of nettai is
incompatible"). The repository github.com/nettaisen/nettai-ai (public) keeps one network for each nettai **line**,
its major and minor version: `<line>/nettai-ai.safetensors`, and `index.json` giving each line's SHA-256 (a check
for a new one is that small file, not the network). Nothing else is published (the user: "don't expose unnecessary
metadata ... i don't know why you need anything other than just the tensors"): the file's metadata is only what
nettai reads to play it (`publish.PLAYED`: the format, the spec hash, the architecture, the levels' knobs, k and
the screen's decision limit, which let a network of another size or timing be published without a release of the
app), nothing of the run it came from. A nettai of 0.11.x plays the 0.11
line's; a change to what the network sees (the observation spec, §3) comes with a new minor
version and a network trained for it, published under its own line, the older lines' left as they are.

- **The app** (the `ai` feature; `screens/practice/weights.rs`) asks for it when Fights is first picked in a run
  (the user's choice: not at start), then shares it: the data folder's copy (`ai/<line>/`) where its SHA-256 is the
  index's, else the line's downloaded (`nettai_platform::http::get`, its `http` feature: reqwest, async, run as a
  `nettai_platform::task` that is cancelled when dropped: tokio's runtime on the desktop, `fetch` in the browser
  (the user: "use reqwest instead of ureq? ureq isn't cancellable or async"),
  from raw.githubusercontent.com, which a page may fetch), checked and kept; with no index, the copy kept. The AI
  plugged in meanwhile is `ai::WaitingAi` on an `ai::Arriving`: till the network arrives it waits as a decision
  waits for a fingerprint (its custom screen held), then it is that network's `NetAi`; one that arrives as an error
  stops it, said in the log. `NETTAI_AI_WEIGHTS=<file>` (the desktop's) plays that file instead (the user: "can
  you also let me override the weights locally for testing?"). No weights are built in, and none are in this
  repository but the test fixture's (crates/nettai-training/tests/fixtures/net.safetensors, small and random),
  which the tests play.
- **Publishing**: `uv run python -m nettai_ai.publish <checkpoint> --repo <nettai-ai checkout> [--line 0.11]
  [--push]` exports it, keeps of its metadata what nettai reads, runs the tests that need trained weights on the file (`NETTAI_AI_WEIGHTS` naming it:
  nettai-training's `weights_to_publish_play_at_three_levels`, nettai's `the_dummy_that_fights_takes_hp_away`, both
  `#[ignore]`d otherwise), writes it and its SHA-256 and commits there ("nettai <line>: new weights"). The first published is the large run's
  update 2,000 (line 0.11), the network the app had built in.

Reading a network is a copy of its floats (a few ms); every `NetAi` of the app's shares one `Arc<Net>`.

Until then the weights were built into nettai-training (`model/nettai-ai.safetensors`, `ai::WEIGHTS`, read once by
`ai::shipped`). The first
is update 1000 of the first cloud run (runs/first, 177 M decisions; 4.56 MB), weak as early weights are. With the
previous action (§7) it was exported again (4.68 MB) from that checkpoint warm-started (`model.warm_start`: the
previous action's columns zero), its metadata the checkpoint's (update, decisions, commit): it plays as it did
(the old file was checkpoint 1,000's tensors exactly; the new one's outputs on the checkpoint's recorded decisions
are the old network's within 3.4e-7, and the Rust pass adds zero columns, the same floats). With the form list's
entries in the keys' place (§4.2, spec f27f9a4a126e990f) it was converted (`nettai_ai.convert`) and exported again
(4.67 MB): its fight, its slots and its value as they were, its entries' head new (it chooses among Crosses at
about random till a run trains it; the `ai` tests play an EXE6 round with it). Then the user asked for the
latest checkpoint (2026-10-24): update 150 of the third cloud run (runs/third, warm-started from the second's 1,150
and trained on choosing Crosses, with the stall cost of §6.4 from its update 100; 31 M decisions of its own;
4.67 MB), exported on the cloud machine. Then, for the user to play (2026-10-24), update 500 of the fourth cloud
run (runs/fourth, warm-started from the third's 350; one decision every 8 ticks, γ 0.999, λ 0.98; 196 M decisions;
its weights' k 8, which the player reads). Then update 750 of the same run (295 M decisions; half its randomized builds'
lists full from 700), which at Hard beat the first run's 1,000 over duplicate deals, 79 won both ways to 36. Then update 1,650 (647 M
decisions; the entropy bonus 0.003 from 1,300, its entropy 2.59 → 2.24), which beat 750 over duplicate deals 92 to 44
at Hard and 82 to 51 at Normal. Then update 2,200 (860 M decisions; entropy 2.01), which beat 1,650 over duplicate
deals 85 to 39 at Hard and 96 to 47 at Normal. Then update 3,750 (1,461 M decisions; entropy 1.76), which beat
2,200 over duplicate deals 108 to 35 at Hard and 94 to 47 at Normal. Then update 5,250 (2,044 M decisions; the
rules' buttons pressable since 5,000: EXE5's Souls, EXE6's Beast Out), which beat 3,750 at Hard 69 to 50, EXE6 most
(33 to 10). Then, for the user to try, the large network's update 700 (run large1, 2.7 M parameters, 274 M
decisions from scratch; its update 500 had beaten the fourth run's update 500 over duplicate deals 100 to 66), then
its update 1,000 (390 M decisions; its update 750 had beaten its 500 93 to 44), then its update 1,500 (585 M; within
14 Elo of the fourth run's update 5,250 over duplicate deals, 57 deals won both ways to 72, ahead in EXE6), then its
update 2,000 (about 780 M decisions; even with 5,250, 73 to 67, Elo 1,188 to 1,183), which became the nettai-ai
repository's first (line 0.11).

The app has the dummy that fights only when built with its `ai` feature (2026-10-26, the user: "have practice
mode ai be a compile time feature flag that's off by default"): nettai-training is an optional dependency of
nettai, the AI's code is `screens/practice/ai.rs`, and without the feature the dummy's Action has no Fights and the
menu no Level (docs/app.md, "The dummy that fights"). `cargo run -p nettai --features ai` plays it; a release built
as packaging/ builds it (`cargo build -p nettai`) has none.

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
  (`Fingerprinter::status`); its look goes through mockups first. Built as the user picked it (board E):
  `PracticeGettingReady` in practice.slint, the box of netplay's connection in its corner, "Getting ready" and the
  chips made of those asked for, a 2-pixel line filling along its foot (docs/app.md, "Getting ready").
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

- **Shape: one GPU and many CPU cores.** The battles run on the cores. The network's updates, about ten times its
  forward pass, run on the GPU. On CPUs alone they cost about 30 times the simulation (measured below), so the
  first plan here, a CPU-only machine, was wrong.
  - **The machine** (the user's pick, 2026-10-23): a Vast.ai instance with an RTX 3090 (24 GB) on an AMD EPYC 7702P,
    with a CPU quota of about 41 cores, Ubuntu 24.04, and PyTorch 2.14 (CUDA 13.0) in `/venv/main`. Its `/workspace`
    is ordinary container storage, lost when the instance is destroyed, so checkpoints are copied back to the Mac.
    Any NVIDIA card from the RTX 20 series on with 8 GB or more would do; cores matter more than the card.
  - **TPUs don't fit** (the user asked, 2026-10-23). They are built for very large matrix work, and this network is
    tiny. XLA handles a recurrent core with changing masks badly.
- **Run size.** A full run is estimated at 2 to 5 billion decisions, about 5 to 15 hours on such a machine. Tuning
  will take several runs.
- **What of the packs goes there: the timings only** (the user: "you don't need the images, just the timing data").
  The battle's loader reads a pack's `content.toml`, `assets.toml`, and each sprite's `animations.json` and
  `sprite.json`. These are the asset names, the animations' timings and the frames' part layouts: 1,648 files,
  12 MB for the three games. No picture or sound is read or sent. `training/cloud/timings.sh` copies just those files out of `data/`. The
  made-up test timings would train a network for a game that isn't quite the real one.
- **Running.** `training/cloud/setup.sh` installs Rust, uv and the bridge on a fresh Ubuntu machine and runs a
  smoke test. `run.sh` starts a run in tmux, with TensorBoard on the machine's localhost; `sync.sh` rsyncs
  checkpoints and logs back.
- **Measured: the network costs more than this section assumed** (`python -m nettai_ai.bench`, the real pool,
  EXE6, 64 envs, one rollout of 15,000 decisions, the default network, the M1 Max, which other sessions kept at
  load 10 to 20):

  | Part | Decisions a second | A core's time a decision |
  |---|---|---|
  | Simulation (`pool.step`, 6 threads) | 74,000 to 77,000 | 0.08 ms |
  | The rollout's inference (4 threads) | 8,600 | 0.47 ms |
  | The update, 3 epochs (4 threads) | 1,820 | 2.2 ms |
  | The update, 3 epochs (8 threads, the pool waiting) | 2,160 | 3.7 ms |

  - The update's time is a third arithmetic (matrix products 22%, attention 13%) and two thirds small ops:
    elementwise 32%, gathers and packing 11%, the name table's gradient 9%, copies 9%, LayerNorm 4%.
  - Variants of one update on the same data (decisions a second): padded fixed shapes 601 (every slot computed:
    packing the sets is worth 3×); 2 epochs 2,712, 1 epoch 5,220; minibatches of 32 and 64 chunks 2,112 and
    2,235; chunks of 32 decisions 2,242; `torch.compile` 705 (graph breaks at the packing's sizes) and 311 padded;
    bf16 autocast 229 (this CPU has no bf16 units; it's for CUDA).
  - Fewer epochs or bigger minibatches don't pay: on FakePool's toy, at 60 s of wall time, 4 epochs had 58% of
    the targets pressed, 3 epochs 51%, 2 epochs 39%, 1 epoch 18%, 3 epochs at 32 chunks a minibatch 40%. With
    the simulation this cheap, reusing a rollout is cheaper than making one. The cheap fix that holds is
    threads: the update on 8 (`run.update_threads`), collection on 4.
  - So on CPUs the network is about 30 times EXE6's simulation (about 7 times EXE5's): a 96-core machine would
    make perhaps 30,000 to 40,000 decisions a second, 2 billion in 15 to 20 hours. The first cloud run should
    time a GPU (L4 or RTX 4090 class) with `bench --device cuda --variants base,padded,compile_padded,bf16,mb64`:
    on a GPU the small ops are kernel launches, which fixed shapes and compilation can fold. If its update makes
    30,000 decisions a second or more, a GPU with 16 to 32 cores beats the 96-core machine. The Mac's own GPU
    (MPS) was slower than its CPU at this size.
- **Measured on the cloud machine** (the user, 2026-10-23: `bench --device cuda`, 36 pool threads, 512 envs, all
  three games, the loop still taking turns):
  - collection 123,717 decisions in 7.21 s (17,000 a second): `pool.step` 2.59 s (47,700 a second), inference
    and Python 4.62 s (26,800 a second);
  - the update, 3 epochs, in learner decisions a second: 16 chunks a minibatch 13,161; 64 chunks 22,136; 2
    epochs 20,876; padded 5,180; compiled and padded 6,578; bf16 11,728;
  - present tokens a decision: general 1, panels 18, objects 4.2, hand 0.5, used 0.1, slots 0.8, rules 4.6,
    forms 2;
  - so the loop made about 10,000 decisions a second, the pool idle through the updates and the GPU through
    collection. The user chose two changes: **overlap collection with the update** and **bigger batches** (§8's
    loop, built 2026-10-23).
- **What changed for it** (§8): the loop overlapped (two threads, two rollouts, the behavior network one update
  behind); the rollouts compact (present tokens only: a decision 9 KB rather than 57, so 1,024 envs' two rollouts
  take about 6 GB of the card and 2,048 envs' about 12 GB, where the dense layout would have needed 15 and 30 GB
  each); each step's tokens gathered on the CPU into pinned memory and copied without waiting, one wait a step;
  the update's chunks chosen on the CPU, its indices copied without waiting and its statistics kept on the device
  to the end; minibatches of 64 chunks by default (8,192 decision slots); `configs/cloud.toml` at 1,024 envs,
  36 pool threads (the container's quota is about 41 cores of the 128 it shows), torch's threads 3 for the
  collector and 2 for the learner, evaluation's pool 2.
- **Measured on the Mac** (`bench --loop`, EXE6, 64 envs, CPU, 16 chunks a minibatch, load 4 to 9 from other
  sessions): the loop makes 2,180 to 2,280 decisions a second against 1,800 taking turns (collect 2.0 s, update
  6.4 s). Here the update is the slower side: the learner is busy 100% and as fast as alone (1.00 to 1.06×), the
  collector busy 41 to 44%, its own work (all but `pool.step`) 1.5 to 1.6× slower than alone, the cores it shares
  with the learner's 8 threads. So the loop runs at the update's own speed and collection is hidden. The split of
  torch's threads (2 or 4 for the collector, 6 or 8 for the learner) moves it by less than the noise;
  `configs/mac.toml` has 2 and 8. The first epoch's mean ratio against the behavior policy is 1.000 to 1.001, its
  KL 0.003 (one update's lag). A rollout here takes 19 MB on the CPU and 187 MB on the device.
- **The GIL.** The threads give it back in every torch op (and the pool's step, and every wait on the device),
  so the learner's ops and the collector's interleave. How much the collector would suffer otherwise, measured on
  16-step rollouts of the same pool: alone 13.5 ms a step; beside 8 torch threads computing (no GIL, only cores)
  20.3 ms; beside a thread running pure Python (holding the GIL, giving it up only every 5 ms switch interval)
  5,063 ms, since each of its thousands of ops a step waits for the GIL again. The loop's 1.6× is the first kind:
  the cores, not the GIL. So nothing on the learner's side may run long stretches of plain Python while the
  collector works (logging and checkpoints are short); on CUDA both threads mostly launch kernels, each op a
  point where the GIL changes hands. `bench --loop` reports each thread's time against alone and the share of
  it on a CPU, which tells the two apart on the cloud machine.
- **Bigger minibatches and fewer epochs on the toy, the loop overlapped** (two seeds, the targets pressed and best
  slots picked over the last 4 updates): at 60 s of the Mac's CPU, 4 epochs of 8 chunks 38% and 36%, 3 epochs
  47% and 42%, 2 epochs 48% and 38%, 3 epochs of 16 chunks 48% and 40%, of 32 chunks 38% and 38%; after 80
  updates, 59% and 43%, 48% and 39%, 49% and 42%, 38% and 39% (3 of 8, 2 of 8, 3 of 16, 3 of 32). Fewer gradient
  steps an update learn less an update; on a CPU, where a step's cost grows with its size, that evens out at 2×
  the minibatch and is lost at 4×. 3 epochs stays. On a GPU a bigger minibatch costs little more than a small one,
  and the cloud's rollouts hold 16 times the toy's chunks, so 64 chunks still make 32 steps an epoch.
- **To measure on the cloud machine**, in `training/`: `NETTAI_PACKS=$PWD/timings uv run --no-sync python -m
  nettai_ai.bench --device cuda --games exe6,exe5,exe4 --envs 1024 --threads 36 --torch-threads 3
  --update-threads 2 --variants base,mb128,mb256 --loop 20` (the update alone at 64, 128 and 256 chunks, then
  20 updates of the loop); `--envs 2048 --variants base --loop 12` beside it for the bigger batch.
- **The first run's stall, and the previous action** (2026-10-24). By update 1,400 the first cloud run had
  stopped getting better: checkpoint 1,400 played 1,000 dead even (53%, 49% and 50% in EXE4, EXE5 and EXE6, about
  160 rounds each), as 1,000 played 500; it beat the passive scripted players 90 to 100% but the masher (random
  presses) only 65 to 75%. A lower entropy bonus (0.01 to 0.003) didn't help. The diagnosis: the network never
  knew what it had pressed. It decides every 4 ticks from a view 6 to 30 ticks old, and its GRU read only the
  pooled view, so under the delay it pressed A again (a chip wasted), stepped again (overshot), lost track of a
  charge: its play stayed incoherent. So the GRU now reads its previous action (§7), and the next run starts from
  the first run's latest checkpoint with it (§8's warm start), playing as that checkpoint did on its first update:
  `training/cloud/run.sh NAME --config configs/cloud.toml --init-from runs/first`. Beside it, a custom screen
  past its decisions now closes in a window too (§4.2): EXE6 rounds between those checkpoints were cut 70% of the
  time by screens held open in the Cross window.

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

   Done (branch learned-ai-app, §10); the practice rows came back with the first trained weights (§10.6, branch
   learned-ai-ui).
5. **Cloud runs.** The full recipe, then the held-out-game test, then the levels' calibration. The weights go in.
6. **Chip fingerprints: in v1** (the coordinator's call, the user wanting multi-step setups learned well). Each chip
   run in a sandbox battle (§3.1): where it hits, when and how hard, what it changes and leaves, and its damage under
   each condition a setup brings; the pairs of chips that work together found from them and seeded into folders.
   Generic, kept by content hash, and a folder's chips take 1 to 4 s on one thread at battle start.

## 13. Risks

- **Forms.** If it rarely learns to choose a Cross or use Beast Out, a small bonus for trying forms early in
  training is the lever (§4.2 makes each one a single choice).
- **Practice settings far from training.** A full gauge every turn, for example. These are randomized in (§6.3).
- **Cost of EXE5 and EXE4 ticks.** They are three times EXE6's. Batches are balanced so EXE6 doesn't dominate.
- **Weights in git.** A few MB a time, replaced rarely.
