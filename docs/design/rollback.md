# Rollback netplay

The engine supports rollback netplay (GGPO-style): each peer runs the whole battle, predicts the other player's
input, snapshots every frame, and when the real input arrives and differs, restores the snapshot of the first
wrong frame and simulates again. This document describes the engine's side of that contract, the rollback
simulator that proves it (crates/bn6-netplay), what the original's per-console ("local side") state means for
it, how sound works under rollback, what it costs, the hazards found, and what the custom screen and scripting
layers must guarantee to keep it working.

Frames are counted from the start of a round; "frame `f`" is the `f`-th tick, and "the state after frame `f`"
is the battle once `f + 1` ticks have run.

## 0. Summary

- **Snapshots**: `Battle` is plain data and `Clone`; a snapshot is a copy (`save_state` / `load_state`), about
  22 KB (8.6 KB inline plus the object pools); saving takes about 2 µs, restoring 2-8 µs (release). The battle's
  content (`Battle::content`, an `Arc<Content>`) never changes and is shared by every snapshot.
- **Digest**: `Battle::digest()` hashes the simulation state (presentation left out) with a platform-independent
  hasher. Peers compare it every confirmed frame. It covers the round's setup, which carries the content's hash
  (`RoundSetup::content`), rather than the content itself.
- **Input**: `Battle::step(&TickInput)`. A `TickInput` is both players' buttons plus the link-closed event at
  the end of a round, which netplay carries in a player's input so both peers step every frame with the same
  record. Both players' custom screens are simulated from their buttons (engine/custom-screen.md).
- **Perspective**: both peers simulate from the same side (`RoundSetup::local_side` is shared setup) and present
  it for their own player (`sound_cues_for`, `banner_for`, `round_end_for`). §2 explains why and lists every
  per-console detail with its decision.
- **Sound**: `cues::CueTracker` turns every simulated tick's cues, tagged with their frame, into Play and Cancel
  actions, so a confirmed cue plays once and a mispredicted one is stopped or undone. bn6-audio takes the actions
  (`BattleAudio::handle_actions`).
- **Results**: synthetic netbattles on the engine's test content (random button mashing, fixed hands) run to the KO
  without a single divergence at latencies of 0 to 10 frames with jitter and input delay, with hundreds to thousands
  of rollbacks each. The golden traces, replayed through two rollback peers at latencies 0, 2, 5 and 10 (plus
  jitter), match the trace on every confirmed frame, with the peers in agreement throughout.
- **Cost**: the worst case, a 10-frame rollback on every rendered frame, costs about 60-90 µs per frame in
  release (under 0.6% of the 16.7 ms budget).

## 1. The model

### 1.1 Inputs

`rollback::TickInput` is everything from outside the simulation that one tick consumes:

```rust
pub struct TickInput {
    pub players: [PlayerTick; 2],   // by side
    pub events: TickEvents,
}
```

Two battles started from the same `RoundSetup` and stepped with the same inputs are in the same state. The
simulation keeps no state outside `Battle`, reads no clock, does no I/O, and uses no floats, statics, interior
mutability, hash-map iteration or addresses (§7).

Each player contributes their share. In bn6-netplay that is `Bn6Input { tick: PlayerTick, events: TickEvents }`;
the frame's record combines both shares (`bn6::tick_input`).

**The custom screen is simulated.** Both players' custom screens run in the engine from their buttons
(engine/custom-screen.md): `PlayerTick` is just the buttons, and what used to be inputs is state:

| Former input | Now |
|---|---|
| `PlayerTick::in_custom` | `custom::Side::in_custom`, carried to the fight by the simulated link (`link::Link`) |
| `TickEvents::local_confirm` | Each player's screen, from their buttons |
| `TickEvents::exchange` | Each player's result (`custom::Side::sent`), arriving 50 + `link_delay` ticks after it is sent |
| `TickEvents::link_closed` | Still an event (the end state's link session closing) |

`TickEvents::recorded` exists only to check against golden traces that lack a player's folder: that player's
screen isn't simulated and the recording supplies what it sent (engine/custom-screen.md §7). The events are part
of the frame's input record: one player's input carries them (the golden-trace replay puts them in player 0's),
both peers receive them like any input, and a peer that predicted "no events" rolls back when they arrive.
Prediction repeats the buttons and never repeats events (`Game::predict` for `Battle`).

The simulated link also delays the fight's view of the buttons by `RoundSetup::link_delay` ticks (4 in the
recordings, as the original's link queue); the custom screens read them at once. This is part of the game, not
of the netplay layer, whose own input delay and prediction come on top.

For synthetic matches, `standin::StandInBattle` closes the link at once at the end of a round; everything else is
the engine, so the players' buttons are the whole input.

### 1.2 Prediction

A missing remote input is predicted as the latest known input of that player (buttons held, events dropped).
Frames are simulated ahead up to `max_prediction` frames past the last contiguous remote input; beyond that the
peer waits.

### 1.3 Snapshots

A snapshot is a copy of the battle: `Battle::save_state()` / `save_state_into()` / `load_state()`, or `Clone`
directly. `Battle` derives `Clone`; every part of its state is plain data (fixed arrays and a few vectors). Copies
share only what never changes: the content the round runs on (`Battle::content`, an `Arc<Content>`: the pack's
battle data and animation timing) and the behaviors handle (code, docs/design/scripting.md). State refers to
content by id, never by reference. So a restored battle continues exactly as the saved one would have: in-repo
tests roll a battle back mid-fight, simulate a wrong future, restore, and compare digests frame by frame.

The per-tick sound cues are part of the copy (they are the cues of the tick that produced the state); a consumer
reads them right after each tick.

### 1.4 The digest

`Battle::digest()` (module `digest`) feeds the simulation state through `digest::StableHasher`:

- every state type derives `Hash`; integers of every width are mixed as little-endian 64-bit words (so `usize`
  and enum discriminants hash the same on 32- and 64-bit targets) with a folded 128-bit multiply;
- a compile-time assertion requires a little-endian target (slices of integers reach the hasher as raw bytes);
- the hasher's output is pinned by a test: changing it changes every digest;
- peers must run the same engine build: the digest covers the state's layout, which changes with the engine.

**Left out, as presentation** (the simulation never reads them): the tick's sound cues, each sprite's `Look`, the
banner's id (`Banner::id`; its lifetime is in), and the objects' `VISIBLE` header flag. With one shared
perspective (§2) these are identical on both peers anyway; leaving them out keeps presentation free to become
per-viewer later without touching desync detection.

**Left out, as immutable input**: the content (`Battle::content`) and the behaviors handle. The digest covers
the round's setup, and the setup carries the content's hash (`RoundSetup::content`, which `Battle::new` checks
against the content it is given), so peers on different content have different digests from the first frame.

**Coverage is enforced.** The `Hash` impls for `Battle`, `Object`, `Sprite` and `Banner` destructure their
structs without `..`: a new field doesn't compile until it is hashed or explicitly left out. Everything else
derives `Hash`, so a new state type that doesn't fails to compile where it is used.

It costs about 20-30 µs (release). Stale data in free object slots is hashed too, because the game reads it (a
freed navi's HP at the end of the round; sprites the game doesn't clear on spawn).

### 1.5 Confirmation and desync detection

A frame is confirmed once both players' inputs for it are known and it was simulated with them (bn6-netplay's
`Observer::confirmed` is called once per frame, in order, with the state after it). The peers compare the digest
of every confirmed frame. The simulator also compares both with a plain lockstep run of the same inputs.

## 2. Perspective: the local side

### 2.1 What the original keeps per console

Each console of the original runs the whole battle, but a few things depend on which side it is: the local navi
appears at once at the intro while the other queues for a fade-in, a blinded player's console hides the other
side's navi and effects, the telop says whose chip it is, the result banner and music are the local
player's, the charge sounds and the "own navi hit" sound are the local player's, the low-HP music follows the
local navi, and the round's result and score are "won/lost" from the local side. The engine reproduces this for
`RoundSetup::local_side`, because the golden traces are recorded on one console and compare exactly that.

Replaying both traces with `local_side` flipped (same inputs) and comparing the whole state every other frame
shows where the two consoles' states differ:

| State | Frames (machgun round 1 / soundmod round 1) |
|---|---|
| The navis' intro phase, fade-in queue, `VISIBLE` flags and sprite animation | 2 to 48 |
| The faded-in navi's `timer`, left at 2 by the fade and not rewritten until much later | 18 to 520 / 18 to 3,120 |
| The Beast Out lock-on marker's `VISIBLE` flag (only its owner's console shows it) | while a navi is in Beast Out (machgun round 1 from 642) |
| Banner id: chip names (0x4C/0x50) and the result banner | whenever shown |
| `fight.state` (WIN/LOSE), `fight.result`, `round.result`, `round.wins`/`losses` | from the KO on |
| The low-HP music latch and the sound cues (before the engine kept both sides', §2.3) | when they fire |
| `local_side` | always |

Nothing else differs: positions, HP, RNG, panels and hands agree throughout.

### 2.2 Decision: one shared simulation perspective

**Both peers simulate from the same `local_side`** (by convention 0, the host), whichever side they present.
`local_side` is part of the shared setup, like the battle settings and the RNG seed; each peer presents the
shared simulation for its own player, the *viewer*.

The alternative was each peer simulating its own console with a digest that normalizes the perspective-dependent
fields away. The table above rules it out: the intro leaves the navis' `timer` different for hundreds to
thousands of frames, so a normalized digest would have to ignore a real simulation field of every navi for most
of a round, and the intro phases and queue besides. That would blind desync detection exactly where state
differs. (The test `the_local_side_is_part_of_the_shared_setup` shows two peers with different sides disagreeing
from the intro's first frames.) With a shared perspective the whole state is identical on both peers and the
digest covers all of it. Peer-specific presentation is derived per viewer instead.

Related shared setup: the battle settings. Netbattle settings come in pairs that differ only in which navi the
actor list spawns first (for example entries 0 and 1), which decides object slots and update order; both peers
must use the same entry. The content is shared setup too: `RoundSetup::content` is the hash (`Content::hash`) of
the content the round runs on, so peers whose setups agree run the same data.

### 2.3 Each per-console detail

| Where | What depends on the side | Decision | Status |
|---|---|---|---|
| `battle.rs` `low_hp_music` | The pinch music follows the local navi's HP | Compute both: a latch per side, `Pinch` cues per viewer | Done |
| `battle.rs` `fight_result` | Victory or defeat music | Compute both: the winner's viewer gets `WINNER`, the other `LOSER` (link battles) | Done |
| `charge_glow.rs` `charge_sound` | Only the charging player hears an A-charge | Compute both: `play_sound_for(alliance, ...)` | Done |
| `player/status.rs` hit sound | 0x6B for the local navi, 0x6D otherwise | Compute both: each viewer hears its own | Done |
| `player/input.rs` | "Can't" sound for SELECT without enough per-side gauge (battle flag 0x40 mode) | Compute both: the pressing player's viewer | Done |
| `dimming.rs` `telop` | The telop: banner 0x4C (own) / 0x50 (other's) | Presentation: `Battle::banner_for(viewer)` swaps them | Done |
| `battle.rs` `fight_result` | Result banner of the local navi, won or lost | Presentation: `banner_for(viewer)` picks the viewer's navi's | Done |
| `battle.rs` `round_result`, `finish_round`, `chain_next_round` | Won/lost, wins/losses, `BattleResult`, `SetScore` | Shared (identical on both peers, relative to `local_side`); presentation: `round_end_for(viewer)` | Done |
| `battle.rs` `finish_round` | `exit_hp`: the local navi's HP after the round | Shared; per-viewer is the viewer's navi's HP (read it from `stats`/the object) | Documented |
| `player/entry.rs` | Which navi appears at once and which fades in | Shared: it is object state (phase, timers) and gates the intro; the non-local viewer sees the mirror image of the original (its own navi fades in) | Accepted, cosmetic |
| `charge_glow.rs` `viewer_sees`, `obstacle.rs` `update_visibility`, `player/status.rs` blind visibility, `dimming.rs` `show_user` | A blinded local player doesn't see the other side's navi, effects and obstacles (`VISIBLE`) | Should be presentation: record "hidden from a blinded viewer" per viewer instead of clearing `VISIBLE` for the local one | Open (cosmetic; the digest leaves `VISIBLE` out) |
| `lockon_marker.rs`, `charge_glow.rs` `shown_to_side` | The Beast Out lock-on marker and the A-charge glow show only on the owner's console | Same as above | Open (cosmetic: the other viewer sees the local player's marker, not its own) |
| `battle.rs` custom screen (`local_confirm`, `round.status`, `custom_ui`) | The local player's custom-screen progress; the gauge task restarts 11 ticks after the local confirmation | Both players' screens are simulated (`custom`); the gauge restarts when either player sends and when the screen closes | Done |
| RNG1 (the per-console stream) | Folder shuffle and other local decisions | Not in the engine: each player's shuffled folder is round setup (`RoundSetup::players`); the custom screen draws no RNG | Done |
| `RoundSetup::low_hp_music_latched` | The local console waited for the link at init | Per-console quirk of the recording; netplay rounds start unlatched | Documented |

## 3. Presentation under rollback

### 3.1 Draw the newest state

A frontend draws the peer's newest state (predicted past the confirmed frame). Visual effects are objects in the
state, so a re-simulation that removes or moves one corrects the picture on the next frame (a visible pop at
worst). The frontend's own presentation state (the HUD remembers whether the gauge was on, whether the round was
over) must tolerate jumps back and forth.

### 3.2 Sound: frame-tagged cues

Sound cues are the one event-like output: a re-simulated tick produces its cues again, and a mispredicted tick
produced cues that didn't happen. `cues::CueTracker` handles that:

- the netplay layer tells it about every rollback (`rolled_back(frame)`) and every simulated tick
  (`simulated(frame, cues)`, first time or again) and confirmation (`confirmed(frame)`);
- a cue plays as soon as a tick first makes it, predicted or not (waiting for confirmation would delay every
  sound by the latency);
- a cue a re-simulated tick makes again is recognised as the one already played if it was played for a frame at
  most `tolerance` frames away (a corrected input often moves an event by a frame or two), and not played again;
- a played cue that the re-simulation doesn't make again, within the tolerance, is cancelled.

The result is a list of `CueAction::Play(cue)` / `Cancel(cue)`. bn6-audio's `BattleAudio::handle_actions`
plays the plays like `handle` and takes back cancels: a sound effect stops if its player is still playing it
(`m4aSongNumStop`), a music change goes back to the previous music (restarted), a pinch switch is switched back.
`bn6_netplay::bn6::CueFeed` connects a peer to a tracker for one viewer.

The synthetic tests check, for each peer, that plays minus cancels equals the cues of the confirmed frames, cue
by cue. (On the synthetic battle as it was before it moved to the test content, at 10 frames of latency a battle
of 7,726 frames played 542 and 558 cues on the two peers, of which 1 and 16 were cancelled predictions.)

### 3.3 Per viewer

`Battle::sound_cues_for(side)` is what that side's player hears (the engine records both); `sound_cues()` stays
the local side's, which the golden sound recordings check. A peer feeds its tracker with its own player's cues.

## 4. The rollback simulator

crates/bn6-netplay is generic over a `Game` (advance on two inputs, digest, `Clone` for snapshots, prediction):

- `Peer`: one peer's session: both players' inputs by frame, prediction, a snapshot of every unconfirmed frame,
  rollback to the first wrong frame and re-simulation, confirmation, statistics, and an `Observer` for
  presentation and checks;
- `network::Link`: a one-way link with latency and jitter (packets can overtake each other);
- `sim::Match`: two peers over two links plus a lockstep reference run. Each wall-clock frame every peer decides
  its input `input_delay` frames ahead and sends it, then receives what arrived and updates. Every confirmed
  frame's digest is compared across the peers and with the lockstep run; a panic in a peer is reported, marked
  speculative if the lockstep run gets through that frame.

`bn6` implements `Game` for `Battle` (the engine's input record, events carried in the inputs) and `standin`
provides the link closing, a MegaMan built in code, a netbattle setup on given content with given folders and a
seeded button masher.

## 5. Results

### 5.1 Synthetic netbattles (in this repository)

`crates/bn6-netplay/tests/rollback.rs`: two navis with 300 HP on the battle settings 0 of the engine's test content
(`content::testing`, hand-authored, not BN6's data). Its chips are made up but run the engine's own actions: side
0's folder holds a level-3 GunDelSol, an eraser navi chip, a level-1 GunDelSol, an invisibility dimming chip and a level-3
GunDelSol over and over (GunDelSol is action 0x37; the invisibility freeze 0x15 with subtype 1, as Invisibl; the
eraser navi chip 0x1B with subtype 5, as EraseMan), side 1's GunDelSols only (no Crosses or Beast Out). Both players
mash (held buttons change every four frames on average: a direction, A, L or R; B and START are never pressed, see
§7.2), and the mashing drives their custom screens too. Three seeds, each under every configuration:

| Latency + jitter | Input delay | Seed 1 / 2 / 3: frames to the KO | Rollbacks per peer (seed 3) | Deepest rollback | Diverged |
|---|---|---|---|---|---|
| 0 | 0 | 4,848 / 7,490 / 6,204 | 0 | 0 | never |
| 1 + 1 | 0 | same | ~1,390 | 2 | never |
| 2 + 1 | 0 | same | ~1,400 | 3 | never |
| 5 + 2 | 0 | same | ~1,500 | 7 | never |
| 10 + 3 | 0 | same | ~1,550 | 13 | never |
| 10 + 2 | 3 | same | ~1,450 | 10 | never |

Every battle runs to the end with both peers' digests equal to each other and to the lockstep run on every frame,
and ends the same way as without rollback. The other tests: the engine's own input record with the recorded events
riding in player 0's input (latencies 3 and 8, in sync), confirmation order, and the two negative tests below. (With
500 HP a mashed battle can reach the 15th custom screen, whose turn timer ends in the damage judge, which the engine
doesn't have yet.)

### 5.2 How long before a divergence, and why

With the engine as it is, never: no configuration diverged. Two deliberate breaks show the checks work and what
would break it:

- **Peers simulating from their own side**: out of sync from the intro's first frames (the fade-in, §2).
- **State outside the snapshot** (a counter shared between a game and its snapshots, as an `Rc` or a static
  would be): out of sync at frame 36 at 4 frames of latency, on the first rollback that re-simulated a frame
  reading it.

### 5.3 The golden traces under rollback (verification workspace)

The golden-trace suite outside this repository replays each round's recorded inputs through two rollback peers
(both simulating the trace's side, the custom-screen events in player 0's input), up to the frames the plain
replay matches. Every confirmed frame of both peers must match the trace exactly, and the peers must agree:

| Round | Frames | Latency 0 | 2 + 1 | 5 + 2 | 10 + 3 |
|---|---|---|---|---|---|
| machgun 1 | 1,074 | all match | all match (84 rollbacks) | all match (90) | all match (100, depth 13) |
| machgun 2 | 1,331 | all match | all match (64) | all match (72) | all match (77) |
| soundmod 1 | 4,513 | all match | all match (220) | all match (244) | all match (260) |
| soundmod 2 | 6,284 | all match | all match (333) | all match (371) | all match (391) |
| soundmod 3 | 2,566 | all match | all match (280) | all match (307) | all match (338) |

(Rollbacks are counted on player 1's peer, which receives player 0's buttons and the custom-screen events; in
the machgun rounds player 0's peer rolls back far less often, since player 1 changes buttons less.)

Since the engine moved to content packs, the rounds run on the pack extracted from the ROM, and every round still
matches its full length (machgun 1,074 and 1,331, soundmod 4,513, 6,284 and 2,566 frames) on every confirmed frame
at every latency. The rollback counts above are from the earlier run.

## 6. Performance

Worst case per rendered frame: restore a snapshot, simulate 10 frames again saving a snapshot after each, then
the new frame, and digest it. Release build:

| | Synthetic battle (mashing, whole battle) | soundmod round 1 (the 2,000 frames around its busiest frame) |
|---|---|---|
| Restore | 5-8 µs | 4.1 µs |
| Simulate one frame | 1.0-1.2 µs | 0.68 µs |
| Save one frame | 2.1-2.8 µs | 2.5 µs |
| Digest | 26-30 µs | 21 µs |
| **Per rendered frame** | **67-91 µs** | **~61 µs** |

(Synthetic: ranges over repeated runs, measured on the synthetic battle before it moved to the test content.
soundmod: measured on the content pack, after the engine moved to packs; the round's battle is the same.)

That is under 0.6% of the 16.7 ms a frame has at 60 fps. The 99th percentiles (0.2-0.6 ms) and the worst frames (a
few ms) were measured on a machine running other builds (load average 20-38) and are scheduling noise, not engine
work. (Measure with `cargo run --release -p bn6-netplay --example rollback_cost --features trace -- <trace.jsonl>
<pack> [round]`.) The digest dominates and is needed once per confirmed frame, not per re-simulated one.

## 7. Hazards

### 7.1 Found and fixed

- **`Battle` wasn't `Clone`.** It derives `Clone` (and `Debug`) now; nothing in it is shared or external.
- **No digest.** Added, with enforced coverage (§1.4).
- **Inputs had no equality**: `PlayerTick`, `TickEvents` and `CustomResult` derive `PartialEq`/`Eq`/`Hash`, so
  a prediction can be checked against the real input.
- **Per-console sound**: the engine recorded only the local player's cues; it records what each side hears.
- **Per-console presentation read from state**: per-viewer banners and results (§2.3).
- **Re-simulated sound cues**: the cue tracker (§3.2).
- **`&'static` references in state** (`BattleSettings::actors`, a rock's kind): they are ids now
  (`BattleSettings::actors` is an `ActorListId`, a rock's kind a variant id), resolved against the battle's
  `Content`, so a snapshot holds no pointers and a serialized snapshot (spectators, desync reports) is possible.

### 7.2 Found, still open

- **Speculative panics.** The engine panics on content that isn't ported yet. A peer simulating a predicted input
  explores input sequences no player made, so it can reach an unported path the real match never does, and crash
  a peer that is otherwise in sync. The synthetic tests avoid the paths that mashing reaches: the buster (B,
  actions 0x11 and 0x16), cut-ins (a dimming chip during the other side's dimming), ElmntMan's random elements, and Beast
  Out's head and rush (so the synthetic players have no Beast Out or Crosses), and the damage judge after the
  15th turn (so their battles are short). The simulator reports a panic as speculative when the
  lockstep run gets through the frame. For netplay, unported paths must become unreachable (content that can't
  run isn't allowed in a netplay folder) or end the battle deterministically on both peers instead of panicking.
- **Per-viewer visibility**: the blindness rules and the lock-on marker and A-charge glow still decide `VISIBLE`
  for the local side (§2.3). Cosmetic for the other viewer; the digest leaves `VISIBLE` out.

### 7.3 Checked, not a problem

- No floats, statics, thread-locals, `Rc`/`RefCell`/`Cell`, hash maps, clocks or I/O in the simulation (the
  frontend's thread-local panic flag and bn6-audio's floats are presentation; the content's `Arc` and the
  behaviors handle's `Rc` point at immutable data and code, not state).
- No address-dependent behavior: the values the original takes from addresses and registers (list-node addresses
  as a deletion effect's position, register garbage in spawn positions and bug codes) are modeled as fixed values
  or marked unknown.
- Order is list order or slot order everywhere; free slots keep their stale data, which the snapshot copies and
  the digest covers.
- Integer overflow: release builds wrap and debug builds panic, so a debug peer and a release peer could part
  ways on an overflow. Peers must run the same build anyway, and the trace suite and these tests run with
  overflow checks, so the paths they cover don't overflow.

## 8. What the other layers must guarantee

### 8.1 The custom screen

Done (engine/custom-screen.md): what follows is what it guarantees.

- **Driven by buttons only.** Its outputs (the hand, the transformation, confirmation timing) come from the
  players' buttons and simulated state, inside `Battle`. Then `TickEvents` and `PlayerTick::in_custom` go away
  and each player's buttons are the whole input.
- **Both sides, all the time.** Each player's screen is simulated on both peers: cursor, selection, confirmation
  and the timers that follow it (`custom_ui`, `round.status` bit 2, the gauge restart 11 ticks after the
  confirmation) must exist per side, or be defined from shared state, never "the local player's". Which screen a
  viewer sees is presentation.
- **No per-console RNG.** The original shuffles the folder with RNG1, a per-console stream, and exchanges the
  result. Either draw from a shared stream seeded in the setup, or treat each player's shuffle as that player's
  input.
- **No link waits in the simulation.** The original's link exchange takes frames; in the port the exchange is
  the simulation's own, deterministic, and doesn't depend on network timing.
- **Everything in `Battle`,** hashed (the destructuring in `digest.rs` will ask for new fields).

### 8.2 Scripting (content)

The rules of the core/content boundary (docs/design/core-content-boundary.md, §5.2) are what rollback needs:

- all content state lives in engine-owned typed storage inside `Battle`, copied by snapshots and covered by the
  digest; a script VM's heap, globals, closures or coroutines hold no battle state between calls;
- integers only; one simulation RNG stream through the core; list or slot order, never hash-map iteration;
- outputs (cues, looks) are write-only;
- content is immutable during a battle and identified by a hash both peers compare before the match (the
  battle data's hash is in the shared setup, `RoundSetup::content`);
- bounded work that fails the same way on every peer;
- no perspective in simulated state: a script that wants "the local player" asks for the viewer at presentation
  time;
- re-running a tick must be free of side effects outside `Battle`: scripts run many times per frame under
  rollback (the worst case above is 11 ticks per rendered frame), so they must also be cheap.

### 8.3 Round chaining

`RoundEnd::NextRound` hands over the next round's settings and score; the next `Battle` also needs its RNG seed
and navi stats, which the original's init exchange provides. A netplay session must derive them from shared data
(the previous round's state, or values exchanged before the match), and keep rolling back across the boundary or
confirm it before starting the next round.
