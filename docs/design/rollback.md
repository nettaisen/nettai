# Rollback netplay

The engine supports rollback netplay: each peer runs the whole battle, predicts the other player's input,
speculates ahead of the frames both players' inputs are known for, and when the real input arrives and differs,
goes back to the last confirmed state and simulates again. The netcode core is getgud, Tango's rollback library;
crates/nettai-netplay implements its `World` for the battle. The players' inputs travel on rennet, Tango's netplay
transport, over UDP (or any datagram channel). This document describes the engine's side of that contract, how
getgud's model maps onto the engine, the protocol and the transport, the simulator that proves them, what the
original's per-console ("local side") state means for it, how sound works under rollback, what it costs, the hazards
found, and what the custom screen and scripting layers must guarantee to keep it working.

Frames are counted from the start of a round; "frame `f`" is the `f`-th tick, and "the state after frame `f`"
is the battle once `f + 1` ticks have run. getgud's tick `t` is the state after `t` ticks.

## 0. Summary

- **Netcode**: getgud (a workspace dependency from its repository, the revision pinned in Cargo.lock) keeps the
  input queues, the settled state, the speculative tail, promotion and rollback, and the clock skew. nettai-netplay
  supplies its `World`, `BattleWorld`: one peer's battle, its player's side, snapshots and prediction (§4).
- **Protocol**: rennet (a workspace dependency likewise) carries each player's inputs to the other peer once and in
  order over datagrams that are lost, reordered and duplicated, every frame resending what isn't acknowledged, so a
  lost datagram's inputs come with the next one. nettai's protocol (`protocol`) names what a player's stream holds:
  a tick's buttons and event flags (one byte for A, B and the directions), a payload for the rare event with data,
  and the round and match markers; and the frame's meta, the sender's tick advantage. A frame costs 6 bytes at no
  latency, 9 at 2 frames, 17 at 5 and 28 at 10 (the unacknowledged window grows with the round trip) (§4.6).
- **Transport**: a `Datagram` trait (send, and take what arrived, never waiting); UDP for direct play
  (`nettai-frontend --play --host PORT` / `--join ADDR:PORT`), a WebRTC data channel later. A handshake checks
  the protocol, the engine and the content, swaps what each player brings and agrees the seed (§4.7).
- **Snapshots**: `Battle` is plain data, `Clone` and `Send`; a snapshot (`save_state` / `load_state`) is a boxed
  copy, `Send` as getgud requires. About 22 KB (8.6 KB inline plus the
  object pools); saving takes about 2-3 µs, restoring 3-5 µs (release). The battle's content
  (`Battle::content`, an `Arc<Content>`) never changes and is shared by every snapshot.
- **Digest**: `Battle::digest()` hashes the simulation state (presentation left out) with a platform-independent
  hasher. Peers compare their settled states' digests. It covers the round's setup, which carries the content's
  hash (`RoundSetup::content`), rather than the content itself.
- **Input**: `Battle::step(&TickInput)`. A `TickInput` is both players' buttons plus the link-closed event at
  the end of a round, which netplay carries in a player's input so both peers step every frame with the same
  record. Both players' custom screens are simulated from their buttons (engine/custom-screen.md).
- **Perspective**: both peers simulate from the same side (`RoundSetup::local_side` is shared setup) and present
  it for their own player (`sound_cues_for`, `banner_for`, `round_end_for`). §2 explains why and lists every
  per-console detail with its decision.
- **Sound**: `cues::CueTracker` turns every simulated tick's cues, tagged with their frame, into Play and Cancel
  actions, so a confirmed cue plays once and a mispredicted one is stopped or undone. The peer's world reports
  every tick it simulates; nettai-audio takes the actions (`BattleAudio::handle_actions`).
- **Results**: synthetic netbattles on the engine's test content (random button mashing, fixed hands) run to the KO
  without a single divergence at latencies of 0 to 10 frames with jitter and present delay, and with 10% to 25% of
  the datagrams lost (in bursts) and 5% to 10% duplicated, with thousands of rollbacks each; through an outage of
  two seconds; across a set's rounds; and over real UDP on loopback, in two threads and in two processes. The golden
  traces, replayed through two getgud sessions at latencies 0, 2, 5 and 10 (plus jitter), clean and lossy, match the
  trace on every confirmed frame, with the peers in agreement throughout (§5).
- **Cost**: the worst case, a 10-frame rollback on every rendered frame, costs about 60-80 µs per frame in
  release, the same as before getgud (under 0.5% of the 16.7 ms budget).

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

Each player contributes their share. In nettai-netplay that is `PlayerInput { tick: PlayerTick, events: TickEvents }`;
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
of the netplay layer, whose own present delay and prediction come on top.

For live netplay and synthetic matches, `standin::StandInBattle` closes the link at once at the end of a round;
everything else is the engine, so the players' buttons are the whole input.

### 1.2 Prediction

A missing remote input is predicted as the latest known input of that player: buttons held, events dropped
(`Game::predict`, which getgud calls through `World::predict` per remote slot, only where the real input hasn't
arrived). A peer presents the frame `present_delay` ticks behind its newest local input; the ticks between the
last confirmed one and that frame are speculated on predicted input. getgud sets no limit on how far: clock sync
keeps the peers' leads even, and a host adds a stall guard (§4.1).

### 1.3 Snapshots

A snapshot is a copy of the battle: `Battle::save_state()` / `save_state_into()` / `load_state()`. `Battle`
derives `Clone`; every part of its state is plain data (fixed arrays and a few vectors). Copies share only what
never changes: the content the round runs on (`Battle::content`, an `Arc<Content>`: the pack's battle data,
animation timing and scripts). State refers to content by id, never by reference. So a restored battle continues
exactly as the saved one would have: in-repo tests roll a battle back mid-fight, simulate a wrong future,
restore, and compare digests frame by frame.

The content's runtime (the Luau VM that runs its scripts) is not part of a battle: each thread keeps runtimes in
a cache keyed by the content hash the round's setup carries, and every runtime made from the same content behaves
the same (docs/design/scripting.md, docs/design/content-model-v2.md §7.3). So a battle and its snapshots are `Send`
by construction, as getgud requires of the states it keeps, and a restored snapshot steps on any thread. The
battle is boxed in the snapshot: getgud moves saved states into and out of its buffers more often than it makes
them, and a battle is 8.6 KB inline.

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

**Left out, as immutable input**: the content (`Battle::content`). The digest covers
the round's setup, and the setup carries the content's hash (`RoundSetup::content`, which `Battle::new` checks
against the content it is given), so peers on different content have different digests from the first frame.

**Coverage is enforced.** The `Hash` impls for `Battle`, `Object`, `Sprite` and `Banner` destructure their
structs without `..`: a new field doesn't compile until it is hashed or explicitly left out. Everything else
derives `Hash`, so a new state type that doesn't fails to compile where it is used.

It costs about 20-30 µs (release). Stale data in free object slots is hashed too, because the game reads it (a
freed navi's HP at the end of the round; sprites the game doesn't clear on spawn).

### 1.5 Confirmation and desync detection

A frame is confirmed once both players' inputs for it are known. getgud folds confirmed frames into the *settled
state*, the authoritative state built from confirmed input rows only, and returns the rows (`Advance::confirmed`)
once each, in order, as they settle; a settled frame is never simulated again. After each advance that settled
frames, the peer digests its settled state (`Session::settled_state`), and the peers compare. One advance can
settle several frames (a burst of remote input) and getgud keeps only the latest settled state, so a check that
needs every confirmed frame follows the world instead: the world reports every frame it simulates
(`Observer::simulated`), and the last simulation of a frame before it settles is the one on the confirmed inputs.
The simulator compares both peers' settled digests with each other and with a plain lockstep run of the same
inputs, and checks that each peer's confirmed rows are the inputs the players decided.

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
| RNG1 (the per-console stream) | Folder shuffle, ChpShufl's re-deal, and what else advances it (camera shakes, the emotion window) | Each player's shuffled folder is round setup (`RoundSetup::players`); each player's console RNG is simulated in `Battle::consoles`, seeded from the setup (`PlayerSetup::console`), and in the digest (custom-screen.md §8) | Done |
| `RoundSetup::low_hp_music_latched` | The local console waited for the link at init | Per-console quirk of the recording; netplay rounds start unlatched | Documented |

## 3. Presentation under rollback

### 3.1 Draw the presented frame

A frontend draws the frame getgud presents (`Advance::frame`, `present_delay` ticks behind the newest local input:
speculated past the confirmed frames when the remote input lags further). Visual effects are objects in the
state, so a re-simulation that removes or moves one corrects the picture on the next frame (a visible pop at
worst). The frontend's own presentation state (the HUD remembers whether the gauge was on, whether the round was
over) must tolerate jumps back and forth.

### 3.2 Sound: frame-tagged cues

Sound cues are the one event-like output: a re-simulated tick produces its cues again, and a mispredicted tick
produced cues that didn't happen. `cues::CueTracker` handles that:

- the netplay layer tells it about every rollback (`rolled_back(frame)`) and every simulated tick
  (`simulated(frame, cues)`, first time or again) and confirmation (`confirmed(frame)`). Only the world sees
  the first two (getgud shows the host the presented frame and the settled state, not the ticks in between), so
  the peer's `BattleWorld` reports them to its observer; the host reports the third after each advance, from the
  count of rows getgud settled;
- a cue plays as soon as a tick first makes it, predicted or not (waiting for confirmation would delay every
  sound by the latency);
- a cue a re-simulated tick makes again is recognised as the one already played if it was played for a frame at
  most `tolerance` frames away (a corrected input often moves an event by a frame or two), and not played again;
- a played cue that the re-simulation doesn't make again, within the tolerance, is cancelled.

The result is a list of `CueAction::Play(cue)` / `Cancel(cue)`. nettai-audio's `BattleAudio::handle_actions`
plays the plays like `handle` and takes back cancels: a sound effect stops if its player is still playing it
(`m4aSongNumStop`), a music change goes back to the previous music (restarted), a pinch switch is switched back.
`nettai_netplay::battle::CueFeed` is the observer that connects a peer to a tracker for one viewer: the peer's world
holds it and the host shares it (`BattleWorld::with_observer` with an `Rc<RefCell<CueFeed>>`, since getgud owns
the world). It also keeps each unconfirmed frame's cues from its latest simulation, which are the confirmed
frame's cues once it settles.

The synthetic tests check, for each peer, that plays minus cancels equals the cues of the confirmed frames, cue
by cue. The behaviour is the same as on the rollback peer nettai-netplay had before getgud: at 10 + 3 frames of
latency, seed 1's battle (11,912 frames) played 588 and 607 cues on the two peers, of which 75 and 93 were
cancelled predictions; the old peer played 587 and 608 and cancelled 74 and 94.

### 3.3 Per viewer

`Battle::sound_cues_for(side)` is what that side's player hears (the engine records both); `sound_cues()` stays
the local side's, which the golden sound recordings check. A peer feeds its tracker with its own player's cues.

## 4. Netplay on getgud and rennet

Each peer runs a getgud `Session` (Tango's rollback core) on a `World` that nettai-netplay implements for the battle.
getgud keeps the input queues and matches them into confirmed rows, speculates, promotes or rolls back, keeps the
settled state and computes the clock skew; it has no game logic and speaks of one local player and remote slots.
The inputs go between the peers on rennet (Tango's netplay transport, §4.6), over a datagram channel (§4.7).

### 4.1 getgud's model on the engine

| getgud | Here |
|---|---|
| `World` | `BattleWorld<G, O>`: one peer's live battle (a `Game`), its player's side, the tick it is parked at, and an observer |
| `World::step(local, remotes)` | `Game::step([side 0's input, side 1's input])`: `local` goes to the world's side, the one remote slot to the other |
| `World::Input` | `PlayerInput` for `Battle` (buttons and the frame's events), the buttons for `StandInBattle`; `Default` is the input before any has arrived (`initial_remotes`) |
| `World::State`, `save`, `load` | `BattleState`: a `Snapshot` (§1.3) and its tick. `load` copies nothing and reports no rollback when the world is parked at that tick already: getgud loads the settled state before simulating confirmed rows even when nothing was speculated past it, and then the world is that state |
| `World::predict` | `Game::predict`: buttons carry on, events happen once |
| `World::recycle` | Not implemented: `Battle`'s `clone_from` is the derived one, which reuses no allocation, so pooling snapshots would save nothing |
| Present delay | `NetConfig::present_delay`, the input delay: a peer presents the frame that many ticks behind its newest local input, so that much of the lead needs no prediction |
| Settled state, `Advance::confirmed` | The confirmed state and rows (§1.5): the digest checks, and the confirmation the sound feed and the trace checks need (`Observer::confirmed`) |
| Speculative tail, promote or roll back | The frames a peer speculates to present. A prefix whose predictions held is promoted without simulating it again; from the first wrong prediction, getgud loads the settled state and simulates the rest again. `last_misprediction_depth` is the number of frames it threw away |
| `skew`, `local_tick_advantage` | Clock sync: every frame carries the sender's advantage as of its newest input (the protocol's meta, §4.6); each input arrives with the freshest one seen (`add_remote_input(slot, input, advantage)`), and a peer that runs ahead stalls frames (§4.3) |
| `matchable`, `local_queue_length` | The stall guard: a peer with `max_lead` unconfirmed local inputs waits, unless remote input it has can still be matched |

### 4.2 What nettai-netplay implements

- `world`: `Game` (a battle stepped on both players' inputs by side, and its prediction), `Observer`
  (`rolled_back`, `simulated`, `confirmed`, implemented for `&mut`, `&RefCell` and `Rc<RefCell>` of an observer
  so that the world and the host can share one), `BattleWorld` and `BattleState`. `BattleWorld::session` makes the
  session.
- `bn6`: `Battle` as a `Game` on the engine's input record (`PlayerInput`, `tick_input`), and the sound feed
  (`CueFeed`).
- `standin`: `StandInBattle` as a `Game` on the buttons alone (what live netplay plays); a MegaMan built in code, a
  netbattle setup on given content with given folders, and a seeded button masher.
- `protocol`: nettai's rennet protocol: the elements, the meta, their codecs, the horizon, and `WireInput`, a
  game input's wire form (buttons, flags, payload), for the buttons alone and for `PlayerInput` (§4.6).
- `wire`: byte codecs for the engine types that travel: a recorded custom-screen result (a `PlayerInput`'s
  payload), a folder, a Cross list, a game, the content's hash. Structs are written field by field, destructured
  without `..`, so a new field doesn't compile until it is written.
- `link`: `InputLink`, one peer's end of the exchange on rennet's `OutStream`/`InStream`: inputs pushed with the
  tick advantage, the datagram to send, the datagrams received turned into inputs in order, and what went by (sizes,
  copies, reordering, the round trip, the other's loss as the receiver sees it).
- `peer`: `Peer`, a getgud session and its link, and the host's frame (below): clock sync, the stall guard, and a
  set's rounds on one stream (§4.6).
- `transport`: the `Datagram` trait, UDP, the handshake and the `Connection` it makes (§4.7), and an in-memory pair
  for tests.
- `network`: a seeded datagram network, one direction of it: latency, jitter (each datagram on its own, so jitter
  reorders), loss alone or in bursts, duplication, outages.
- `sim::Match`: two peers over two networks plus a lockstep reference run (§4.3).

A host does each frame what the simulator does for a peer (`Peer`): take the datagrams that arrived
(`Peer::receive`), hold the frame if running ahead or at the stall guard (`Peer::wait`), else decide its player's
input (`Peer::decide`, which pushes it on the link with `local_tick_advantage()`); send the frame's datagram
(`Peer::datagram`, every frame, held or not: it carries the acks and the window); if it decided, `advance`
(`Peer::advance`), tell its observer what settled, and draw `frame.state.battle()`. The frontend's netplay driver
does exactly this (docs/frontend.md §2).

### 4.3 The simulator

`sim::Match` runs two `Peer`s in one process, over a simulated datagram network each way (`network::Network`,
seeded: the same configuration and seed replay exactly). Each wall-clock frame, every peer takes the datagrams that
have arrived; every peer that doesn't wait (the stall guard, or a clock-sync stall) decides its player's input for
its next tick; every peer sends its datagram; then every peer that decided takes what arrived meanwhile (with no
latency, the other's input of the same frame) and advances. A link that breaks (a gap past the horizon, §4.6) tears
the match down, and the report says which peer, when and why.

**Clock sync.** A peer adds its skew up every frame while its presented frame speculates
(`speculation_balance() >= 0`; until then the present delay absorbs the lead) and stalls a frame for every 60 of
it (`SKEW_PER_STALL`), never two in a row, so that it keeps sending its advantage and the two peers can't wait on
each other. That is a frame-rate adjustment of a frame a second per tick of skew; a peer one frame ahead shows a
skew of 2. Stalling at once on any positive skew doesn't work: a stall shows in the skew in full only a round
trip later (the peer's own lead drops at once, the advantage the other peer reports only once it has missed the
input and said so), and jitter makes the skew noisy, so the peers took turns stalling, a third of all frames at 1
to 10 frames of latency. At the rate above, peers that start even stall about one frame in 500 to 1,000, and a
peer that starts 6 or 20 frames ahead gives the frames back (stalls, and waits at the stall guard) until both
speculate as deep as from an even start.

**Checks.** Every advance: each confirmed row must be the inputs the two players decided for that tick, and the
settled state's digest must equal the lockstep run's at that tick and the other peer's where it settled the same
tick. The observers see every frame each world simulates and every settle. A panic in a peer is reported with the
frame being simulated, marked speculative if the lockstep run gets through that frame. The inputs end at
`max_frames`: near it a peer's present delay grows so that it presents and speculates nothing past the end, while
its settled state still reaches it.

**Statistics**, per peer: rollbacks, the deepest, and the frames they threw away (getgud's
`last_misprediction_depth`); frames simulated for the first time; the deepest and the mean speculation
(`speculation_balance`); stalls and waits at the guard. Per link: datagrams and bytes, elements a datagram, copies
received, the loss the receiver estimates; per network: datagrams lost, the longest run lost, duplicated, reordered.

### 4.4 What was deleted

nettai-netplay's own GGPO-style session: `Peer` (both players' inputs by frame, prediction, a snapshot of every
unconfirmed frame, rollback to the first wrong frame and re-simulation, confirmation, statistics), the `Game`
trait it drove (snapshots by `Clone`, `advance` on both inputs, `digest`, `is_over`, `blank_input`) and
`HasBattle`. `Observer::confirmed` used to come once per frame with that frame's state; it now comes once per
settle with the settled state. `NetConfig::input_delay` and `max_prediction` became `present_delay` and the stall
guard's `max_lead`.

With rennet: `network::Link`, the simulated one-way link that delivered in order (a packet that would overtake the
one before it waited for it, as an ordered channel would), and `NetConfig::link`. The inputs now travel as rennet
frames over a datagram network that keeps no order, and the clock sync and stall guard moved from the simulator
into `Peer`, which a real host drives the same way.

### 4.5 What doesn't fit getgud

- **`State` must be `Send`.** A battle is `Send`: the content's runtime lives in a per-thread cache, not in the
  battle (§1.3).
- **The host can't reach the world.** `Session` has no accessor for its `World`, so what only the world sees
  (every simulated tick, for sound and checks) reaches the host through a handle the two share. A
  `Session::world()` (and `world_mut()`) would do without it.
- **Only the latest settled state.** An advance that settles several ticks keeps only the last state, so
  per-frame checks follow the world instead (§1.5). Returning the settled tick with each row would at least say
  which rows were promoted and which simulated again.
- **Loading the parked state.** getgud loads the settled state before simulating confirmed rows even when the
  world is parked there; its own test world skips that, as `BattleWorld` does, but `World::load`'s contract
  doesn't say it may.
- **No end of input.** A session runs as long as it is advanced; to settle the last ticks without speculating
  past them, the simulator raises the present delay at the end.

### 4.6 The protocol on rennet

getgud needs each remote player's inputs in tick order, each once. The network delivers datagrams in any order,
some twice, some never. rennet (Tango's netplay transport, a git dependency pinned in Cargo.lock like getgud) is the
layer between: each peer has an `OutStream` for its player's inputs and an `InStream` for the other's, and every
datagram is one rennet `Frame`: a base sequence number, the cumulative ack of the other stream (as a signed
difference from the base), a meta, and a run of elements. The out-stream keeps every element the other peer hasn't
acknowledged (at least the last two) and sends all of them in every frame; the in-stream puts what arrives back in
order, drops copies and acknowledges the contiguous prefix. A lost datagram's elements come with the next one, so a
single loss costs one frame, not a round trip. rennet is pure (no I/O, no clock) and generic over a `Protocol`,
which says what an element and the meta are. nettai's (`protocol::Netplay`):

| Element | What | Wire form (a LEB128 head; bit 0 tells a tick from the rest) |
|---|---|---|
| `Tick { held, flags }` | One tick of a player's input: the buttons (ten), and the game's flags for the tick's events | `(buttons << 1) | flags << 11`: buttons in the wire order A, B, right, left, up, down, SELECT, START, R, L, so that a tick with only A, B and directions is one byte, with any button two, with flags up to three |
| `Payload(chunk)` | Up to 32 bytes that the next tick carries besides, in order before it | head `(len + 1) << 1 | 1`, then the bytes |
| `RoundEnd` | The sender's round is over: what follows is the next round's | `0x01` |
| `MatchEnd` | The sender left | `0x03` |

The **meta** is the sender's tick advantage as of its newest input (getgud's `local_tick_advantage`, a signed
LEB128: one byte), which the receiver hands getgud with each input it delivers (rennet keeps the freshest, so a
reordered old frame can't set it back). An ack-only frame (nothing new to send while the peer waits) still carries
the window, the ack and the meta.

**A game's input on the wire** is `protocol::WireInput`: its buttons, flags and payload. The stand-in battle's input
is the buttons, with no flags (live netplay's: the end of a round is derived in the game). `PlayerInput`'s events
are flags: the link closing, and for each side whose recording lacks a folder, the recorded screen status and
whether a result comes; a recorded result is the payload (a `CustomResult` in `wire`'s form, 130 to 170 bytes,
four to six chunks), once a custom screen. Only the golden-trace replays have recorded events; they go through the same
protocol as live play.

**Byte cost.** A frame is the header (the base, two bytes from the 128th element to the 16,383rd, three after;
the ack, one; the meta, one) and the window: one element a tick, a byte with A, B and the directions. The window is
what the other peer hasn't acknowledged, so it grows with the round trip. Measured on the synthetic battles (§5.1),
the mean frame per peer:

| Network | Elements a frame | Bytes a frame | With the transport (1 + UDP/IPv4's 28) |
|---|---|---|---|
| no latency | 2.0 | 6.1 | 35 |
| 1 + 1 | 3.0 | 7.2 | 36 |
| 2 + 1 | 5.0 | 9.4 | 38 |
| 5 + 2 | 11.8 | 16.6 | 46 |
| 10 + 3 | 22.4 | 28.0 | 57 |
| 10% lost, no latency | 2.4 | 6.6 | 36 |
| 10% lost, 2 + 2 | 6.1 | 10.5 | 40 |
| 15% lost, 5 + 3 | 13.1 | 18.0 | 47 |
| 25% lost, 10 + 4 | 24.1 | 30.0 | 59 |

At 60 frames a second each way, that is about 2 to 3.5 kB/s with the headers. (The tick flags of a golden-trace
replay's player 0, whose recording lacks the other player's folder, make its ticks two bytes; the results add four
to six chunks a custom screen.)

**The horizon** (`protocol::HORIZON`, 240 elements: four seconds) is the widest gap the in-stream accepts: an element
that many past the first missing one can't be recovered in time, and the link breaks (`LinkError::HorizonExceeded`).
The out-stream keeps at most that many unacknowledged elements. getgud has no limit of its own: a peer's stall guard
(`max_lead`) bounds how far its player's input runs ahead of the other's, and since each side may lead by that much
a gap reaches at most twice the stall guard; `protocol::max_lead` keeps the stall guard under half the horizon,
less a margin for payloads and markers (`PeerConfig::new` checks; the frontend's stall guard is 30). So two peers
that keep it never open a gap past the horizon, and a gap past it means a peer that doesn't, which tears the match
down. A link that just goes quiet doesn't break: both peers wait at their stall guard, sending their windows, and
when the link comes back the match goes on (§5.1); the frontend gives up after 10 seconds of silence.

**Rounds.** A set's rounds are one getgud session each (a session counts ticks from its start), on one rennet
stream. When a peer's settled state is over, it ends the round (`Peer::end_round`): it pushes `RoundEnd` and starts
the next round's session from the next round's setup, which both peers build from the settled end state and the
shared setup (`nettai-frontend`'s `driver::next_round_setup`: the round's end hands over the settings and the score;
each player's folder is shuffled again by their console's RNG where the round left it; the battle's RNG is drawn
from the first round's and the round number). The other player's inputs past their round's end, which the peer
predicted past the end before it noticed, are dropped (they come before their `RoundEnd`); their next round's
inputs, if they started it first, wait for this peer's next round. The protocol's `MatchEnd` tells the other peer
that a player left (`Peer::leave`).

### 4.7 The transport and the handshake

`transport::Datagram` is all a peer needs of a channel: send a datagram to the other peer, and take the next one
that arrived, neither ever waiting; nothing is assumed of delivery. `transport::Udp` is direct play: a host binds a
UDP port on every IPv4 interface and takes the first Hello's sender as the other peer (connecting the socket to it,
so that nothing else is read); a joiner connects to the host's address. The "port unreachable" a UDP socket reports
for a datagram the other end didn't take (the host isn't up yet) is not an error.

Every datagram on the channel starts with a byte that says what it is (`transport::Kind`): a protocol frame, a
Hello, or a refusal. A frame needs that byte because a handshake message can come late or twice, into the match,
and must not be read as a frame. (A WebRTC peer could keep the handshake on a reliable channel of its own and do
without the byte.)

**The handshake** (`Connection::host`, `Connection::join`): the joiner sends its `Hello` every 100 ms until it has
the host's; the host answers each Hello with its own. A Hello says:

- the protocol's version (`protocol::VERSION`) and the engine's (the crate version: peers must run the same engine,
  since the digest covers the state's layout and the simulation must be the same code);
- the content's hash (`Content::hash`: the definitions, scripts and rule tables, and the asset names and animation
  timing the battle reads from the pack);
- the role (host or joiner) and the side's half of the seed (a nonce from the clock and the process);
- what the player brings, as bytes the frontend encodes (its `Offer`: a folder, a game, the Crosses, the patch cards,
  and from the host a forced stage; the language is each player's own and isn't sent).

A side that gets a Hello it can't play with (another protocol or engine, other content, the same role) sends a
refusal with the reason three times and stops; the other stops on reading it, so both say what differs. Once a
side has the other's Hello the match is on: both have the seed (`Connection::seed`, mixed from both halves) and
both players' setups, and build the same round. If the host's answer was lost, the joiner keeps sending its Hello,
and the host, in the match, answers it again; the first frame from the other side is the sign it has ours. The
frontend checks the other player's setup against the content (`netplay::Offer::check`: a legal folder of the
content's chips, Crosses of MegaMan's, the content's patch cards, a link battle stage) before building the round.

### 4.8 What WebRTC would need

Tango plays its matches over a WebRTC data channel opened unordered and without retransmits, set up through a
signalling server (matchmaking by a link code). To plug in here:

- a `Datagram` on such a channel: `send` posts a message on the channel; `try_recv` takes one from a queue the
  channel's message callback fills (the channel's library is asynchronous; the peer and its session stay on the
  frontend's thread, which polls the queue every frame);
- the signalling: an offer and an answer (SDP) and ICE candidates exchanged through a server, before the channel
  opens; the host and joiner roles follow who made the link code;
- the handshake as it is, over the channel (or on a second, reliable channel, without the kind byte on frames);
- NAT traversal (STUN, and TURN when that fails) comes with WebRTC; UDP direct play needs a forwarded port instead.

Nothing above the `Datagram` changes: the protocol, the peer and the frontend's driver are the same.

## 5. Results

### 5.1 Synthetic netbattles (in this repository)

`crates/nettai-netplay/tests/rollback.rs`: two navis with 300 HP on the battle settings 0 of the engine's test content
(`content::testing`, hand-authored, not BN6's data). Its chips are made up but run the engine's own actions: side
0's folder holds a level-3 GunDelSol, an eraser navi chip, a level-1 GunDelSol, an invisibility dimming chip and a level-3
GunDelSol over and over (GunDelSol is action 0x37; the invisibility freeze 0x15 with subtype 1, as Invisibl; the
eraser navi chip 0x1B with subtype 5, as EraseMan), side 1's GunDelSols only (no Crosses or Beast Out). Both players
mash (held buttons change every four frames on average: a direction, A, B, L or R; START is never pressed), and the
mashing drives their custom screens too. Three seeds, each under every configuration, over getgud sessions whose
inputs go on rennet over the simulated datagram network:

| Network | Present delay | Seed 1 / 2 / 3: frames to the KO | Rollbacks per peer (seed 3) | Deepest rollback | Clock-sync stalls per peer (seed 3) | Diverged |
|---|---|---|---|---|---|---|
| no latency | 0 | 7,456 / 14,222 / 16,156 | 0 | 0 | 0 | never |
| 1 + 1 | 0 | same | ~1,990 | 2 | 7 | never |
| 2 + 1 | 0 | same | ~3,640 | 3 | 7 | never |
| 5 + 2 | 0 | same | ~3,490 | 7 | 14 | never |
| 10 + 3 | 0 | same | ~3,350 | 13 | 32 | never |
| 10 + 2 | 3 | same | ~3,480 | 9 | 19 | never |
| 10% lost (bursts 30%), 5% duplicated, no latency | 0 | same | ~135 | 6 | 12 | never |
| 10% lost (bursts 30%), 5% duplicated, 2 + 2 | 0 | same | ~3,370 | 8 | 32 | never |
| 15% lost (bursts 40%), 5% duplicated, 5 + 3 | 0 | same | ~3,165 | 12 | 56 | never |
| 25% lost (bursts 50%), 10% duplicated, 10 + 4 | 0 | same | ~2,930 | 19 | 123 | never |

("Bursts 30%": once a datagram is lost, the next is lost 30% of the time, so about 13% are lost in all, in runs of
up to 8; at 25% and 50%, about 34%, in runs of up to 14. Jitter reorders the datagrams: about one in ten arrives
after a later one at 5 + 2.)

Every battle runs to the end with both peers' settled digests equal to each other and to the lockstep run at every
advance, and ends the same way as without rollback; each peer's sound plays every confirmed cue once. Rollbacks come
at about the same rate per frame as over the ordered link this replaced (a fifth of the frames at 10 + 3, the
deepest 13), though the battles are longer now (the test content changed since). A lost datagram's inputs come with
the next one, so loss costs a frame per datagram lost in a row: the deepest rollback grows by about the longest run,
and the peers stall a little more for clock sync. getgud rolls back less often than the peer nettai-netplay had
before it: it checks predictions as rows settle, promotes the prefix that held, and catches up on several arrivals in
one rollback.

The other tests: the engine's own input record with the recorded events riding in player 0's input (latencies 3 and
8, in sync), observers seeing every simulated and settled frame, clock sync (a peer that starts 6 or 20 frames
ahead), the two negative tests below, and:

- **an outage**: nothing gets through either way for two seconds (120 frames, at 3 + 1): both peers wait at their
  stall guard (114 frames each), the match goes on after it, in sync, to the KO;
- **a gap past the horizon**: with a horizon of 32 under a stall guard of 40 (one that doesn't fit, §4.6), one
  direction fails for five seconds; the peer that hears nothing waits at its guard while the other runs on to its
  own, and when the link comes back the gap is wider than the horizon: that peer tears the match down at tick 300,
  where both settled states agree. With the full horizon, the same match goes on;
- **a set's rounds**: two `Peer`s over a lossy network (4 + 2, 10% lost, 5% duplicated) play a best-of-three set
  as a frontend does, ending each round when their settled state is over and starting the next on the same stream:
  the settled states agree in every round, and when the set is over one player leaves and the other hears it.

(With 500 HP a mashed battle can reach the 15th custom screen, whose turn timer ends in the damage judge, which the
engine doesn't have yet.)

**Over real UDP** (`tests/udp.rs`): two peers on loopback, each with its own socket, shake hands and play 600 ticks of
a mashed battle at their own pace (a frame each 4 to 5 ms), once in two threads and once in two processes (the test
binary run again as each peer); their settled states agree at every tick both settled. The frontend's test plays two
`NetPlayer`s on loopback on BN6's content (the handshake, each player's own loadout, 900 ticks of mashing), and they
agree too. Two frontends in their windows, one hosting and one joining on loopback, play with a 18 ms round trip; the
joiner sees the battle from its side, with its own custom screen.

### 5.2 How long before a divergence, and why

With the engine as it is, never: no configuration diverged. Two deliberate breaks show the checks work and what
would break it:

- **Peers simulating from their own side**: out of sync from the intro's first frames (the fade-in, §2).
- **State outside the snapshot** (a counter shared between a game and its snapshots, as an `Rc` or a static
  would be): out of sync at tick 37 at 4 frames of latency, on the first rollback that re-simulated a frame
  reading it.

### 5.3 The golden traces under rollback (verification workspace)

The golden-trace suite outside this repository replays each round's recorded inputs through two getgud sessions
(both simulating the trace's side, the custom-screen events in player 0's input), up to the frames the plain
replay matches. Every frame either peer confirms (its last simulation before it settles, which the world reports)
and every settled state must match the trace exactly, and the peers must agree:

| Round | Frames | Latency 0 | 2 + 1 | 5 + 2 | 10 + 3 |
|---|---|---|---|---|---|
| machgun 1 | 1,074 | all match | all match (78 rollbacks) | all match (82) | all match (81, depth 12) |
| machgun 2 | 1,331 | all match | all match (62) | all match (61) | all match (61) |
| soundmod 1 | 6,728 | all match | all match (305) | all match (305) | all match (301, depth 13) |
| soundmod 2 | 6,857 | all match | all match (337) | all match (340) | all match (337) |
| soundmod 3 | 3,088 | all match | all match (306) | all match (306) | all match (305) |

(Rollbacks are counted on player 1's peer, which receives player 0's buttons and the custom-screen events; in
the machgun rounds player 0's peer rolls back far less often, since player 1 changes buttons less.) An advance
settles one frame at no latency and about two at 10 + 3, where late packets hold later ones up: soundmod round 1
settled 3,748 times for its 6,728 frames. Clock sync stalled each peer 17 frames of that round's 6,758 wall
frames.

## 6. Performance

The worst case per rendered frame, a 10-frame rollback every frame, through the battle's `World` the way getgud
drives a rollback: load the settled state, step the corrected tick and the 10 speculated after it saving each, and
digest the new settled state. On soundmod round 1 (the 2,000 frames around its busiest frame), release build:

| | Before getgud (clone-based peer) | getgud (`BattleWorld`) |
|---|---|---|
| Restore | 2.83 µs | 3.04 µs |
| Step one frame | 2.68 µs | 2.94 µs |
| Save one frame | 2.10 µs | 1.95 µs |
| Digest | 21.2 µs | 21.2 µs |
| **Per rendered frame** | **77.3 µs** | **80.1 µs** |
| Per rendered frame, each frame's fastest of 7 alternating runs | 60.0 µs | 60.4 µs |

(The verification workspace's `soundmod_rollback_cost`, before and after the port, on a machine running other
builds; its numbers moved by up to 60% with the load between runs. The last row runs the two paths side by side,
alternating, and takes each frame's fastest of 7 runs, which leaves the scheduling noise out: the two are the
same.) That is under 0.5% of the 16.7 ms a frame has at 60 fps. (Measure with `cargo run --release -p nettai-netplay
--example rollback_cost -- <trace.jsonl> <pack> [round]`.) The digest dominates and is needed
once per settle, not per re-simulated frame.

A real session costs far less than the worst case, since most frames promote their prediction: soundmod round 1
through two sessions at 10 frames of latency (no jitter, the netcode alone) cost 5.7 to 6.1 µs per rendered frame
with getgud and 6.2 to 6.6 µs with the old peer, measured side by side.

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
  a peer that is otherwise in sync. The synthetic tests avoid the paths that mashing reaches: cut-ins (a dimming
  chip during the other side's dimming), ElmntMan's random elements, and Beast Out's head and rush (so the
  synthetic players have no Beast Out or Crosses), and the damage judge after the 15th turn (so their battles are
  short). The simulator reports a panic as speculative when the lockstep run gets through the frame. A panic
  inside `Session::advance` leaves getgud's session between a load and a save, so the match can't go on. For
  netplay, unported paths must become unreachable (content that can't run isn't allowed in a netplay folder) or
  end the battle deterministically on both peers instead of panicking.
- **Per-viewer visibility**: the blindness rules and the lock-on marker and A-charge glow still decide `VISIBLE`
  for the local side (§2.3). Cosmetic for the other viewer; the digest leaves `VISIBLE` out.

### 7.3 Checked, not a problem

- No floats, statics, thread-locals, `Rc`/`RefCell`/`Cell`, hash maps, clocks or I/O in the simulation (the
  frontend's thread-local panic flag and nettai-audio's floats are presentation; the content's `Arc` points at
  immutable data, and the per-thread runtime cache holds code, not state).
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
- **Per-console RNG in the simulation.** The original shuffles the folder with RNG1, a per-console stream, and
  exchanges the result: each player's shuffled folder is setup. ChpShufl's re-deal draws from the same stream
  mid-battle, so each player's console RNG is simulated (`Battle::consoles`): its seed is setup both peers share
  (`PlayerSetup::console`), and everything that advances it is a function of the simulation and that player's
  buttons.
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
and navi stats, which the original's init exchange provides, and each player's shuffled folder and console RNG
(`PlayerSetup::console`; the original's carries on from the last round's `Battle::consoles` through the next
init's folder shuffle). A netplay session must derive them from shared data
(the previous round's state, or values exchanged before the match), and keep rolling back across the boundary or
confirm it before starting the next round. A getgud session has no end of its own: a host ends the round's
session once its settled state is over (`round_end`), and starts the next round's from that settled state and
the shared data.

Done that way (§4.6, "Rounds"): a peer confirms the boundary (its settled state is over) before it starts the
next round, marks its stream, and builds the next round from the settled end state and the setup both peers
agreed (the frontend's `driver::next_round_setup`: the settings and score the end hands over; the first round's
navi stats; each player's folder shuffled again by their console's RNG where the round left it, and the console's
frame counter carried on; the battle's RNG drawn from the first round's and the round number, since the
original's init exchange has no counterpart here).
