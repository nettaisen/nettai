# Rollback netplay

The engine supports rollback netplay: each peer runs the whole battle, predicts the other player's input,
speculates ahead of the frames both players' inputs are known for, and when the real input arrives and differs,
goes back to the last confirmed state and simulates again. The netcode core is getgud, Tango's rollback library;
crates/nettai-netplay implements its `World` for the battle. The players' inputs travel on rennet, Tango's netplay
transport, over a WebRTC data channel (or any datagram channel). This document describes the engine's side of that contract, how
getgud's model maps onto the engine, the protocol and the transport, the simulator that proves them, what the
original's per-console ("local side") state means for it, how sound works under rollback, what it costs, the hazards
found, and what the custom screen and scripting layers must guarantee to keep it working.

Frames are counted from the start of a round; "frame `f`" is the `f`-th tick, and "the state after frame `f`"
is the battle once `f + 1` ticks have run. getgud's tick `t` is the state after `t` ticks.

## 0. Summary

- **Netcode**: getgud (a workspace dependency from its repository, the revision pinned in Cargo.lock) keeps the
  input queues, the settled state, the speculative tail, promotion and rollback, and the clock skew. nettai-netplay
  supplies its `World`, `BattleWorld`: one peer's battle, its player's side, snapshots, prediction and the
  observer that hears everything that happens to the simulation (§4).
- **Protocol**: rennet (a workspace dependency likewise) carries each player's inputs to the other peer once and in
  order over datagrams that are lost, reordered and duplicated, every frame resending what isn't acknowledged, so a
  lost datagram's inputs come with the next one. nettai's protocol (`protocol`) names what a player's stream holds:
  a tick's buttons and event flags (one byte for A, B and the directions),
  and the round and match markers; and the frame's meta, the sender's tick advantage. A frame costs 6 bytes at no
  latency, 9 at 2 frames, 17 at 5 and 28 at 10 (the unacknowledged window grows with the round trip) (§4.6).
- **Transport**: a WebRTC data channel, unordered and without retransmits (nettai-rtc: native on the `rtc` crate,
  the browser's on wasm32), met in a room of the signaling server (`signaling/`, a Cloudflare Worker: a player asks
  it for a room, it picks the room's code, and the other player joins by that code) or directly (a host's UDP port,
  joined by its address). The two leave the room once their connection is through, having agreed over it on the
  code of a reconnect room; a dropped connection is made again through that room, a new code agreed each time, and
  the match goes on where it was. What goes wrong is
  a kind a host can say to its player in its own words, with the detail apart, for a log (§4.8, §4.9).
- **Before the match**: a lobby checks the protocol, the engine, the game and the content; holds the match's
  settings, which are both players' (the rounds, whether patch cards apply, and whether builds are closed: either
  changes them, both see the same, and both ready marks clear); checks each build as the settings will play it
  (without its patch cards where they are off); says nothing of either player's build; then the handshake swaps
  what each player brings and agrees the seed, each host loads the match, and the first tick runs once both have
  (§4.7).
- **After the match**: the two players are back in the same lobby on the same link, with the same settings and
  nobody ready, each free to bring another build; another match needs no new invite. A match's frames say which
  match they are of, so nothing of the one before reaches the next (§4.7, item 5).
- **Each player's own**: the input delay (4 ticks unless they say, 0 to 10: getgud's present delay, §4.10), and
  what they see of the connection: the ping, how unsteady it is, a quality of four levels, and a countdown while
  it reconnects (§4.11).
- **Snapshots**: `Battle` is plain data, `Clone`, `Send` and `Sync` (checked at compile time); a snapshot
  (`save_state` / `load_state`) is a boxed copy, `Send` as getgud requires, and a whole session on a battle world
  is `Send`, so it can run on a network thread. About 22 KB (8.6 KB inline plus the
  object pools); saving takes about 2-3 µs, restoring 3-5 µs (release). The battle's content
  (`Battle::content`, an `Arc<Content>`) never changes and is shared by every snapshot.
- **Digest**: `Battle::digest()` hashes the simulation state (presentation left out) with a platform-independent
  hasher. Peers compare their settled states' digests. It covers the round's setup, which carries the content's
  hash (`RoundSetup::content`), rather than the content itself.
- **Input**: `Battle::step(&TickInput)`. A `TickInput` is both players' buttons plus the link-closed event at
  the end of a round, which netplay carries in a player's input so both peers step every frame with the same
  record. Both players' custom screens are simulated from their buttons (engine/custom-screen.md).
- **Perspective**: both peers simulate from the same side (`RoundSetup::local_side` is shared setup) and present
  it for their own player (`sound_cues_for`, `banner_for`, `round_end_for`, `visible_to`). §2 explains why and
  lists every per-console detail with its decision.
- **Sound**: `cues::CueTracker` turns every simulated tick's cues, tagged with their frame, into Play and Cancel
  actions, so a confirmed cue plays once and a mispredicted one is stopped or undone. The peer's world reports
  every tick it simulates, every rewind and every tick that settles; nettai-audio takes the actions
  (`BattleAudio::handle_actions`).
- **Failures**: a tick that panics (a content error, or a state the original can't go on from) stops the battle
  (`RoundEnd::Error`) instead of unwinding into getgud. Both peers stop alike on confirmed input; on predicted
  input the stop is rolled back like any speculation (§7.1).
- **Results**: synthetic netbattles on the engine's test content (random button mashing, fixed hands) run to the KO
  without a single divergence at latencies of 0 to 10 frames with jitter and present delay, and with 10% to 25% of
  the datagrams lost (in bursts) and 5% to 10% duplicated, with thousands of rollbacks each; through an outage of
  two seconds; across a set's rounds; and over WebRTC data channels on loopback, directly and through a signaling
server's room, through a three-second outage the connection comes back from. The golden
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
the frame's record combines both shares (`exe6::tick_input`).

**The custom screen is simulated.** Both players' custom screens run in the engine from their buttons
(engine/custom-screen.md): `PlayerTick` is just the buttons, and what used to be inputs is state:

| Former input | Now |
|---|---|
| `PlayerTick::in_custom` | `custom::Side::in_custom`, which the fight reads at once (`RoundState::remote_status`) |
| `TickEvents::local_confirm` | Each player's screen, from their buttons |
| `TickEvents::exchange` | Each player's result (`custom::Side::sent`), arriving 50 ticks after it is sent (its 50 words) |
| `TickEvents::link_closed` | Still an event (the end state's link session closing) |

The events are part
of the frame's input record: one player's input carries them (the golden-trace replay puts them in player 0's),
both peers receive them like any input, and a peer that predicted "no events" rolls back when they arrive.
Prediction repeats the buttons and never repeats events (`Game::predict` for `Battle`).

The engine has no link of its own: the fight and the custom screens read a tick's buttons at once. The only
delay is the netplay layer's present delay, which is the player's input delay (`NetOptions::input_delay`, 4 ticks
unless they say: each player's own, changed during a match; §4.10), and prediction. The original's link queue (4 ticks in the recordings) is the recordings' cable,
which their replay reproduces at its boundary (battle-flow.md §6.1).

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
and `Sync` by construction, as getgud requires of the states it keeps, and a restored snapshot steps on any
thread. Compile-time checks keep it so: nettai-battle's for `Battle`, `Snapshot` and the input records, and
nettai-netplay's for its states and inputs, `BattleWorld` with its sound feed, and a whole `Session` on it (a
session is `Send` when its world is). The battle is boxed in the snapshot: getgud moves saved states into and out
of its buffers more often than it makes them, and a battle is 8.6 KB inline.

The per-tick sound cues are part of the copy (they are the cues of the tick that produced the state); a consumer
reads them right after each tick.

Netplay is not the snapshots' only user: a replay's viewer keeps one every four seconds of the set as it plays,
with each console's HUD state and sound beside it, and seeks by going on from the nearest (docs/frontend.md §8,
"Seeking"). A seek restores a state the replay passed through and plays the recorded buttons on from it, so it asks
the simulation nothing a tick doesn't; its tests hold the digest after a seek to straight playback's. (A snapshot of
an EXE6 link battle measured there is 32 KB.)

### 1.4 The digest

`Battle::digest()` (module `digest`) feeds the simulation state through `digest::StableHasher`:

- every state type derives `Hash`; integers of every width are mixed as little-endian 64-bit words (so `usize`
  and enum discriminants hash the same on 32- and 64-bit targets) with a folded 128-bit multiply;
- a compile-time assertion requires a little-endian target (slices of integers reach the hasher as raw bytes);
- the hasher's output is pinned by a test: changing it changes every digest;
- peers must run the same engine build: the digest covers the state's layout, which changes with the engine.

**Left out, as presentation** (the simulation never reads them): the tick's sound cues, each sprite's `Look`, the
banner's id (`Banner::id`; its lifetime is in), the objects' `VISIBLE` header flag and who else sees them
(`Object::sight`, §2.3), and the message of an engine error that stopped the battle (that it stopped is in; a
content runtime's message can quote addresses). With one shared perspective (§2) these are identical on both
peers anyway; leaving them out keeps presentation free to be per-viewer without touching desync detection.

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
once each, in order, as they settle; a settled frame is never simulated again. Each row (`getgud::Confirmed`) says
its tick, whether its speculation was promoted (every prediction held, so its snapshot became the settled state
as it was) or it was simulated as it settled (`Settlement`), and the settled state after it where getgud kept one:
a promoted row's snapshot, and the last of the rows an advance simulated. The rows simulated before that one were
stepped without a save. getgud tells the world the same rows (`World::settled`), and `BattleWorld` passes them to
its observer (`Observer::confirmed`, per frame, with the state where there is one), after its last simulation of
each: the last simulation of a frame before it settles is the one on the confirmed inputs (a promoted frame's
speculation, or the re-simulation that settled it). A peer digests its settled state (`Session::settled_state`)
for the peers to compare. The simulator digests every settled state getgud returns with a row and compares it
with the other peer's at that tick and with a plain lockstep run of the same inputs, and checks that each row is
the inputs the players decided for its tick.

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

So are the keys the round's host answers in the battle's place (`RoundSetup::host_keys`: the pause, and L's run
key on the custom screen; docs/engine/battle-flow.md §3.5). They change what a tick does with a button, so they
are setup, not something a host switches as it plays: every match is made with both
(`nettai_match::HOST_KEYS`, in `Match::round`), nothing in the lobby sets them, and two peers that differed would
differ from the first digest (the setup is in it). What a host is told of a START (`Battle::pause_requests`) is
the tick's output, as its sound is: a netplay driver says the local player's alone, from the first simulation of
each tick (their own buttons are never predicted; whether the fight took the press there is the prediction's, as
the picture is), and never again when a rollback simulates the tick anew.

### 2.3 Each per-console detail

| Where | What depends on the side | Decision | Status |
|---|---|---|---|
| `battle.rs` `low_hp_music` | The pinch music follows the local navi's HP | Compute both: a latch per side, `Pinch` cues per viewer | Done |
| `battle.rs` `fight_result` | Victory or defeat music | Compute both: the winner's viewer gets `WINNER`, the other `LOSER` (link battles) | Done |
| `charge_glow.rs` `charge_sound` | Only the charging player hears an A-charge | Compute both: `play_sound_for(alliance, ...)` | Done |
| `player/status.rs` hit sound | 0x6B for the local navi, 0x6D otherwise | Compute both: each viewer hears its own | Done |
| `player/input.rs` | "Can't" sound for SELECT without enough per-side gauge (the chip gate battle, battle flag 0x40) | Compute both: the pressing player's viewer | Done |
| `dimming.rs` `telop` | The telop: banner 0x4C (own) / 0x50 (other's) | Presentation: `Battle::banner_for(viewer)` swaps them | Done |
| `battle.rs` `fight_result` | Result banner of the local navi, won or lost | Presentation: `banner_for(viewer)` picks the viewer's navi's | Done |
| `battle.rs` `round_result`, `finish_round`, `chain_next_round` | Won/lost, wins/losses, `BattleResult`, `SetScore` | Shared (identical on both peers, relative to `local_side`); presentation: `round_end_for(viewer)` | Done |
| `battle.rs` `finish_round` | `exit_hp`: the local navi's HP after the round | Shared; per-viewer is the viewer's navi's HP (read it from `stats`/the object) | Documented |
| `player/entry.rs` | Which navi appears at once and which fades in | Shared: it is object state (phase, timers) and gates the intro; the non-local viewer sees the mirror image of the original (its own navi fades in) | Accepted, cosmetic |
| `perspective.rs` `sees` (`sub_800EB6C`), `player/status.rs` and `obstacle.rs` `update_visibility`, `dimming.rs` `show_user`, the charge glow, the Full Synchro aura, status visuals; content's barrier visual, Reflector shield, BugFix glow and dimming stand-in | A blinded player doesn't see the other side's navi, effects and obstacles (`VISIBLE`) | Presentation, per viewer: each site decides the object's visibility for both viewers (`Battle::hide_from_blind`, `hide_from`; content's `me:hide_from_blind()`), recorded in `Object::sight`; `Battle::visible_to(r, viewer)` reads it. The `VISIBLE` flag stays the local side's, which the recordings check | Done |
| `target_marker.rs`, `charge_glow.rs` | The Beast Out lock-on marker and the A-charge glow show only on the owner's console | Presentation, per viewer: `Battle::hide_from_other_side` | Done |
| The overlays and attachments that take their owner's `VISIBLE` (form, body and idle overlays, ice and bubble visuals, effects that follow; content's attachments, drills, axes, clouds...) | They follow the local side's view of their owner | Presentation, per viewer: `Battle::copy_visibility` (content's `me:copy_visibility(owner)`) copies each viewer's view | Done |
| `player/status.rs` `counter_shader` | The other side's navi blinks blue while it can be countered, to a local player in Full Synchro | Should be a per-viewer look: the shader is the sprite's `Look`, one per object | Open (cosmetic: the other viewer sees its own navi blink when the local player could counter it) |
| content's barrier visual (`hidden_unless_local_megaman`), `heal.rs` and `dimming.rs` trap mark | A barrier's parts hidden unless its navi is the local MegaMan; the trap mark faces the local side's way | Same as above (looks) | Open (cosmetic) |
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
  (`simulated(frame, cues)`, first time or again) and confirmation (`confirmed(frame)`). The peer's
  `BattleWorld` reports all three to its observer: the first two as getgud drives it, the third as getgud tells
  it each row that settles (`World::settled`);
- a cue plays as soon as a tick first makes it, predicted or not (waiting for confirmation would delay every
  sound by the latency);
- a cue a re-simulated tick makes again is recognized as the one already played if it was played for a frame at
  most `tolerance` frames away (a corrected input often moves an event by a frame or two), and not played again;
- a played cue that the re-simulation doesn't make again, within the tolerance, is canceled.

The result is a list of `CueAction::Play(cue)` / `Cancel(cue)`. nettai-audio's `BattleAudio::handle_actions`
plays the plays like `handle` and takes back cancels: a sound effect stops if its player is still playing it
(`m4aSongNumStop`), a music change goes back to the previous music (restarted), a pinch switch is switched back.
`nettai_netplay::battle::CueFeed` is the observer that connects a peer to a tracker for one viewer: the peer's world
owns it (`BattleWorld::with_observer`), and the host reads the actions it collects through the session
(`Session::world`, `BattleWorld::observer`), with nothing shared between the two. It also keeps each unconfirmed
frame's cues from its latest simulation, which are the confirmed frame's cues once it settles.

The synthetic tests check, for each peer, that plays minus cancels equals the cues of the confirmed frames, cue
by cue. At 10 + 3 frames of latency, seed 1's battle (7,456 frames) plays 653 and 691 cues on the two peers, of
which 53 and 65 are canceled predictions. (When nettai-netplay moved onto getgud, the numbers on the engine of
the time were within one or two of the rollback peer it had before.)

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
| `World` | `BattleWorld<G, O>`: one peer's live battle (a `Game`), its player's side, the tick it is parked at, and its observer |
| `World::step(local, remotes)` | `step_game(game, [side 0's input, side 1's input])`: `local` goes to the world's side, the one remote slot to the other; a tick that panics stops the battle (§7.1) |
| `World::Input` | `PlayerInput` for `Battle` (buttons and the frame's events), the buttons for `StandInBattle`; `Default` is the input before any has arrived (`initial_remotes`) |
| `World::State`, `save`, `load` | `BattleState`: a `Snapshot` (§1.3) and its tick. getgud tracks where the world is parked and loads only to move it, so a load always restores and is a rollback for the observer |
| `World::Error` | `Infallible`: a failed tick is battle state (`RoundEnd::Error`), not an error of the session |
| `World::predict` | `Game::predict`: buttons carry on, events happen once |
| `World::recycle` | Not implemented: `Battle`'s `clone_from` is the derived one, which reuses no allocation, so pooling snapshots would save nothing |
| `World::settled` | `Observer::confirmed(frame, state)`: each frame as it settles, with the settled state where getgud kept one |
| `Session::world` | How the host reads the observer (the sound feed's actions, the simulator's counts) |
| Present delay | `NetConfig::present_delay`, the input delay: a peer presents the frame that many ticks behind its newest local input, so that much of the lead needs no prediction. The frontend's is the player's own setting (`NetOptions::input_delay`, 4 by default, 0 to 10, never sent to the other peer: §4.10), changed during a match with `Session::set_present_delay` (`Peer::set_present_delay`, kept for the rounds after) |
| Settled state, `Advance::confirmed` | The confirmed state and rows, each with its tick, how it settled and its state where kept (§1.5): the digest checks, and the rows the simulator checks against what the players decided |
| `Session::drain` | The end of the input (`max_frames` in the simulator): the last ticks settle without anything speculated past them |
| Speculative tail, promote or roll back | The frames a peer speculates to present. A prefix whose predictions held is promoted without simulating it again; from the first wrong prediction, getgud loads the settled state and simulates the rest again. `last_misprediction_depth` is the number of frames it threw away |
| `skew`, `local_tick_advantage` | Clock sync: every frame carries the sender's advantage as of its newest input (the protocol's meta, §4.6); each input arrives with the freshest one seen (`add_remote_input(slot, input, advantage)`), and a peer that runs ahead stalls frames (§4.3) |
| `matchable`, `local_queue_length` | The stall guard: a peer with `max_lead` unconfirmed local inputs waits, unless remote input it has can still be matched |

### 4.2 What nettai-netplay implements

- `world`: `Game` (a battle stepped on both players' inputs by side, and its prediction), `step_game` (how a peer
  steps one: a panic stops the battle), `Observer` (`rolled_back`, `simulated`, `confirmed`; implemented for
  `&mut` of an observer), `BattleWorld`, which owns its observer and tells it all three, and `BattleState`.
  `BattleWorld::session` makes the session.
- `exe6`: `Battle` as a `Game` on the engine's input record (`PlayerInput`, `tick_input`), and the sound feed
  (`CueFeed`).
- `standin`: `StandInBattle` as a `Game` on the buttons alone (what live netplay plays); a MegaMan built in code, a
  netbattle setup on given content with given folders, and a seeded button masher.
- `protocol`: nettai's rennet protocol: the elements, the meta, their codecs, the horizon, and `WireInput`, a
  game input's wire form (buttons and flags), for the buttons alone and for `PlayerInput` (§4.6).
- `wire`: byte codecs for what the handshake carries: its versions, the game, the content's hash, the nonce. (What
  a player brings to the match is the frontend's to encode: a match file's side, as text.)
- `link`: `InputLink`, one peer's end of the exchange on rennet's `OutStream`/`InStream`: inputs pushed with the
  tick advantage, the datagram to send, the datagrams received turned into inputs in order, and what went by (sizes,
  copies, reordering, the round trip, the other's loss as the receiver sees it).
- `peer`: `Peer`, a getgud session and its link, and the host's frame (below): clock sync, the stall guard, and a
  set's rounds on one stream (§4.6).
- `network`: a seeded datagram network, one direction of it: latency, jitter (each datagram on its own, so jitter
  reorders), loss alone or in bursts, duplication, outages.
- `sim::Match`: two peers over two networks plus a lockstep reference run (§4.3).

A host does each frame what the simulator does for a peer (`Peer`): take the datagrams that arrived
(`Peer::receive`), hold the frame if running ahead or at the stall guard (`Peer::wait`), else decide its player's
input (`Peer::decide`, which pushes it on the link with `local_tick_advantage()`); send the frame's datagram
(`Peer::datagram`, every frame, held or not: it carries the acks and the window); if it decided, `advance`
(`Peer::advance`) and draw `frame.state.battle()`, taking the sound actions its world's observer collected since
the last frame (`peer.session().world().observer()`: a read, so the session keeps track of where its world is
parked; `world_mut` would cost it a restore). The world has told the observer what settled: nothing is shared
between the host and the world, and a peer (session, world, observer and link) is `Send`, so it can run on a
thread of its own. A round's session ends once its settled state is over (`round_end`): `Peer::end_round` starts
the next on a new world with its own observer (the frontend starts a new sound tracker; an observer that goes on
is taken from this round's world first, `session_mut().world_mut().observer_mut()`). Where the peers agree on a
last tick (a replay, the simulator's `max_frames`), `Peer::drain` settles every local input (`is_drained`) without
speculating past it. The frontend's netplay driver does exactly this (docs/frontend.md §2).

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

**Checks.** Every advance: each confirmed row must be the inputs the two players decided for its tick, and every
settled state getgud returns with a row (most rows: every promoted one, and the last of each re-simulation) must
have the digest of the lockstep run's at that tick and of the other peer's where it returned the same tick. The
lockstep run steps as a peer does (`step_game`), so a tick that fails on confirmed input stops all three alike.
The observers see every frame each world simulates, every rewind and every frame that settles. The inputs end at
`max_frames`: from there a peer drains its session (`Session::drain`), so its settled state reaches the end as the
other's last inputs arrive, and no peer simulates a tick past it.

**Statistics**, per peer: rollbacks, the deepest, and the frames they threw away (getgud's
`last_misprediction_depth`); frames simulated for the first time; rows promoted; the deepest and the mean
speculation (`speculation_balance`); stalls and waits at the guard. Per link: datagrams and bytes, elements a
datagram, copies received, the loss the receiver estimates; per network: datagrams lost, the longest run lost,
duplicated, reordered.

### 4.4 What was deleted

nettai-netplay's own GGPO-style session: `Peer` (both players' inputs by frame, prediction, a snapshot of every
unconfirmed frame, rollback to the first wrong frame and re-simulation, confirmation, statistics), the `Game`
trait it drove (snapshots by `Clone`, `advance` on both inputs, `digest`, `is_over`, `blank_input`) and
`HasBattle`. `NetConfig::input_delay` and `max_prediction` became `present_delay` and the stall
guard's `max_lead`.

With rennet: `network::Link`, the simulated one-way link that delivered in order (a packet that would overtake the
one before it waited for it, as an ordered channel would), and `NetConfig::link`. The inputs now travel as rennet
frames over a datagram network that keeps no order, and the clock sync and stall guard moved from the simulator
into `Peer`, which a real host drives the same way.

### 4.5 What didn't fit getgud, and how it does now

The first port onto getgud worked around five things. getgud's branch `nettai-fixes` (in the user's repository,
tangobattle/getgud) fixes them where they belong, with tests of its own, and nettai-netplay drops the workarounds
(so it needs getgud from that branch on; Cargo.lock pins the revision):

- **`State` must be `Send`.** A battle always was (the content's runtime lives in a per-thread cache, not in the
  battle, §1.3); now compile-time checks keep it so, in nettai-battle and nettai-netplay. getgud documents that a
  session is `Send` when its world is, and the whole battle session is: the shared handle below is gone, so a
  session can run on a network thread.
- **The host can't reach the world.** getgud has `Session::world` and `world_mut` (after which it reloads its own
  snapshot before it steps the world again), and a hook, `World::settled`, that tells the world each row as it
  settles. The observer is the world's own and hears everything from it; the `Rc<RefCell>` the host and the world
  shared, and the observer impls for `&RefCell` and `Rc<RefCell>`, are gone.
- **Only the latest settled state.** Each row `advance` returns (`getgud::Confirmed`) says its tick, whether it
  was promoted or simulated as it settled (`Settlement`), and the settled state after it where getgud kept one
  (§1.5); getgud keeps a call's promoted states until the next call so the rows can lend them. The simulator
  checks rows by their tick (no counting) and digests every state that comes with one (most rows, where it used
  to digest one per advance), and the trace checks compare each with the trace.
- **Loading the parked state.** getgud tracks where its world is parked and loads only to move it; `World::load`'s
  contract says so. `BattleWorld::load` no longer checks the tick: a load is a rewind.
- **No end of input.** `Session::drain` settles what the queued input confirms without new local input and
  speculates nothing, however far past the present target; `is_drained` says when every local input has settled.
  The simulator drains at `max_frames` instead of raising the present delay at the end.

getgud also stays consistent after a `World` error now: the local input is taken, a rollback settles its rows only
once its steps and save have succeeded, rows settled before the error come back from the next call, and the
session reloads its own snapshot before it next steps a world an error may have left anywhere. The battle world
has no errors of its own (§7.1), but other worlds do.

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
| `RoundEnd` | The sender's round is over: what follows is the next round's | `0x01` |
| `MatchEnd` | The sender left | `0x03` |

The **meta** is the sender's tick advantage as of its newest input (getgud's `local_tick_advantage`, a signed
LEB128: one byte), which the receiver hands getgud with each input it delivers (rennet keeps the freshest, so a
reordered old frame can't set it back). An ack-only frame (nothing new to send while the peer waits) still carries
the window, the ack and the meta.

**A game's input on the wire** is `protocol::WireInput`: its buttons and flags. The stand-in battle's input
is the buttons, with no flags (live netplay's: the end of a round is derived in the game). `PlayerInput`'s one event
is a flag: the link closing. The golden-trace replays go through the same
protocol as live play.

**Byte cost.** A frame is the header (the base, two bytes from the 128th element to the 16,383rd, three after;
the ack, one; the meta, one) and the window: one element a tick, a byte with A, B and the directions. The window is
what the other peer hasn't acknowledged, so it grows with the round trip. Measured on the synthetic battles (§5.1),
the mean frame per peer:

| Network | Elements a frame | Bytes a frame | With the transport (1 + 94) |
|---|---|---|---|
| no latency | 2.0 | 6.1 | 101 |
| 1 + 1 | 3.0 | 7.2 | 102 |
| 2 + 1 | 5.0 | 9.4 | 104 |
| 5 + 2 | 11.8 | 16.6 | 112 |
| 10 + 3 | 22.4 | 28.0 | 123 |
| 10% lost, no latency | 2.4 | 6.6 | 102 |
| 10% lost, 2 + 2 | 6.1 | 10.5 | 106 |
| 15% lost, 5 + 3 | 13.1 | 18.0 | 113 |
| 25% lost, 10 + 4 | 24.1 | 30.0 | 125 |

The transport is the kind byte (§4.7) and WebRTC's framing: SCTP's DATA chunk (16) and common header (12), DTLS's
record with AES-GCM (37) and UDP/IPv4's 28 (and SCTP's padding to 4 bytes, up to 3 more). At 60 frames a second
each way, that is about 6 to 7.5 kB/s with the headers, and SCTP's acknowledgments of the other side's frames. The golden traces cost the same
(§5.3: 5.9 to 28 bytes a frame).

**The horizon** (`protocol::HORIZON`, 240 elements: four seconds) is the widest gap the in-stream accepts: an element
that many past the first missing one can't be recovered in time, and the link breaks (`LinkError::HorizonExceeded`).
The out-stream keeps at most that many unacknowledged elements. getgud has no limit of its own: a peer's stall guard
(`max_lead`) bounds how far its player's input runs ahead of the other's, and since each side may lead by that much
a gap reaches at most twice the stall guard; `protocol::max_lead` keeps the stall guard under half the horizon,
less a margin for markers (`PeerConfig::new` checks; the frontend's stall guard is 30). So two peers
that keep it never open a gap past the horizon, and a gap past it means a peer that doesn't, which tears the match
down. A link that just goes quiet doesn't break: both peers wait at their stall guard, sending their windows, and
when the link comes back the match goes on (§5.1); the frontend gives up after 10 seconds of silence.

**Rounds.** A set's rounds are one getgud session each (a session counts ticks from its start), on one rennet
stream. When the tick a round ended on settles, a peer ends the round (`Peer::end_round`): it pushes `RoundEnd` and
starts the next round's session from the next round's setup, which both peers build from the battle at that tick
(the frontend's world observer keeps the first state it simulates with the round over, and drops it on a rewind to
it or before; a battle ticks on after its round's end, so a state settled later, which one peer can reach in the
same advance and the other not, would make another next round) and the shared setup (`nettai_match::Set::after`, which offline play asks too: the round's end hands over the settings and the score;
each player's folder is shuffled again by their console's RNG where the round left it; the battle's RNG is drawn
from the first round's and the round number). The other player's inputs past their round's end, which the peer
predicted past the end before it noticed, are dropped (they come before their `RoundEnd`); their next round's
inputs, if they started it first, wait for this peer's next round. The protocol's `MatchEnd` tells the other peer
that a player left (`Peer::leave`).

### 4.7 The transport, the lobby and the handshake

The transport is the host's: nettai-netplay and nettai-frontend have no socket of their own. The host has a link to
the other player (nettai's is a WebRTC data channel, nettai-rtc's `Link`, §4.8) and gives its datagrams to a
`lobby::Lobby`, which the two players are in from the moment they meet until one leaves: before a match, through
it and after it, for the next (below). The frontend's `NetPlayer` plays on a `netplay::Channel` (send a frame to
the other player, take the next one that came, neither ever waiting; nothing is assumed of delivery; and, if it can
tell, that it is down while it is being made again, for how long and with how long left, `reconnecting`, §4.8).
The lobby gives a match its channel (`Lobby::channel`), so a match's frames go through the lobby too. A host may
still bring a channel of its own for a match (nettai's old `netplay::Framed`, which takes the link for one match
and is the end of it).

Every datagram on the link starts with a byte that says what it is (`lobby::Kind`): a match's frame, a lobby
message, a Hello, a Reveal, a refusal, or a loading message. A frame needs that byte because a handshake message
can come late or twice, into the match, and must not be read as a frame. After it a frame says which match it is
of, by the match's seed (four bytes; `lobby::frame_datagram`): the same two players play one match after another
on one link, and a frame of the match before, come late, must be nobody's. The lobby's messages share the frames'
unreliable channel: the lobby says its part every 100 ms until it is answered, and a Hello or a Reveal that comes
twice is answered again, so a reliable channel of their own would add nothing.

**The lobby, the handshake, the loading and the way back** are nettai-frontend's `lobby::Lobby`, without IO: the
host feeds it every datagram that arrives (`receive`), polls it each frame (`poll`), and sends what it hands back
(`outgoing`). This is version 1 of the protocol (`protocol::VERSION`; a peer of another version is told from the
first field of its lobby messages and refusals, and nothing after that field is read of it).

1. **Lobby.** Each peer says its lobby state when it changes and every 100 ms:
   - **The compatibility fields**: the protocol's version, the engine's (the crate version: peers must run the same
     engine, since the digest covers the state's layout and the simulation must be the same code), the game (a
     match is of one), the content's hash (`Content::hash`) and the role. A peer that can't play with the other
     refuses at once (below).
   - **The match's settings** (`lobby::Settings`): the game, the round wins the set is first to (a byte, 1 to 10:
     1 a single battle, 2 the original's triple battle) and its `2n − 1` rounds (each round's stage and background
     by name or left to the seed),
     `patch_cards` (whether the players' patch cards apply: on unless a player turns them off) and
     `closed_builds`. They are **the match's, not a player's**. Either player changes them (`Lobby::propose`, or
     `set_first_to`, `set_patch_cards` and `set_closed_builds`), the other peer adopts the change, so both lobbies
     show the same
     (`Lobby::settings`), and both players' ready marks clear. It sits on the proposal model the lobby had: each
     peer still says the settings it holds, and they are agreed when both are ready on the same. What is new is
     how two peers' settings become the same without the players doing it. Every change carries a **stamp**, one
     above every stamp its peer has seen; a peer that hears settings of a higher stamp than its own takes them (and
     says them back at once, so the other sees them taken). Where both players changed them at once, the stamps
     are the same and the settings differ: the host's stand and the joiner takes them, as a joiner coming into the
     lobby takes the host's (both start at stamp 0). The peers agree only on settings of the same stamp.
   - **Whether its player is ready** to play the settings as they stand, and **the player's name** (shown, never
     taken for who they are; its control characters dropped, 16 characters at most).
   - **A clock**, and the other's clock said back with how long it was held: the round trip (§4.11).

   **Patch cards off, and a build as it will be played.** The user's instruction: "for netbattle settings you
   should also have a toggle for patch cards on/off". With `patch_cards` off neither player's cards apply: each
   side plays as it is without them. One function says what that is, `Settings::as_played(content, side)`: the side
   itself with cards on, and with them off `nettai_match::Side::without_patch_cards`, which empties the list a
   game's rules keep a side's cards in and changes nothing else. That list is the setup field `patch_cards` in
   every game that has cards (EXE6's and EXE5's a list of cards, EXE4's of a card and its slot), and its name is
   all Rust knows of patch cards (`nettai_match::facts::PATCH_CARDS`): no game is named, and a game whose setup has
   no such field has no cards to leave out. Everything that needs a side "as this match plays it" asks that one
   function:
   - **The match is made of the sides so**, in `netplay::netplay_setup`, where the agreed match is made of the two
     sides on both peers: the set, its first round's digest (which both say as they load, item 4) and the match a
     recording keeps (`Agreement::m`) hold each side without its cards. `Agreement::sides` are the sides as
     played, and `Agreement::patch_cards()` says which way it was.
   - **A build is checked so.** A build that keeps a rule only by a card of its own (a sixth Mega chip that a
     card's Mega folder +1 allows) breaks it in a match without cards. `Settings::checked(content, side)` is a
     build's check of the side as played: `nettai_match::build::Checked::of`, the check the build screens ask
     (what a match's checks say of any side, and the rules' `build_check` besides: EXE4's card out of its slot),
     so the lobby and a build screen never give two answers of one build. A peer checks its own player's build
     when the lobby is made, when the build changes and whenever the settings change, its own change or one
     adopted: `Lobby::problems()` is what is wrong, typed (`nettai_match::Wrong`, with the setup field and entry
     each is of), and a player whose build has problems can't be ready (`Lobby::set_ready` says false, and the
     mark stays clear). Turning the cards on again, or bringing another build, checks again.
   - **A side is revealed so** (item 3): cards that don't apply never cross the wire.

   Held to it by tests: nettai-match's `a_side_without_its_patch_cards` (the three games: the list empty, no
   other fact changed, the round's stats those of the build before it had the card); nettai-frontend's
   `with_patch_cards_off_a_match_is_played_without_them` (two EXE6 builds with cards agree a match with cards
   off: its seed, sides, first digest and 1,500 ticks of battle are those of the same builds with their cards
   taken out by hand and cards on, its recording holds the sides without cards, and its Reveals are the same
   bytes), `in_each_game_a_match_without_patch_cards_is_of_the_builds_without_them`, and
   `a_build_legal_only_by_its_cards_cant_be_played_without_them` (six Mega chips by StarMan's card: ready with
   cards on; with them off "six Mega chips, five at most", typed, no ready mark, and a program that reveals it
   anyway refused, with the problem on both sides).

   **Nothing of a player's build is in it** (the user's instruction: "the lobby shouldn't show build name or which
   game, since that's private information to the player"; "the build name should not really be visible across the
   wire or in replays or anything"). The lobby says none of a build's name, its version or its navi, and a test
   holds a peer's lobby messages to the same bytes whatever build its player brings.
   A player who changes their build in the lobby says nothing by it: if they were ready they no longer are, which
   is all the other sees, and the other's own ready mark stays. A build's name is sent in no message at all: what
   the Reveal carries is the side, which is its facts and has no name.
2. **Hello.** A peer that sees the settings agreed (both ready, on the same settings of the same stamp) commits:
   its Hello carries the compatibility fields, the lobby revisions it agrees to and the round wins the set is
   first to, in the clear, and SHA-256 of the agreed settings' hash and the bytes of the Reveal it will send. The settings' hash
   covers `patch_cards` and `closed_builds`, so both commit to them. A peer reveals nothing before it has the other's Hello; a change
   in the lobby takes the commitment back (and a Reveal already sent is spent: the next one has a fresh nonce).
3. **Reveal.** The lobby revisions it is of (as its Hello said them), a peer's half of the seed (a 64-bit nonce
   from the clock and the process) and its side as the match will play it (the player's setup, in nettai-match's
   binary; `Settings::as_played`). A Reveal of other revisions is one of an agreement that was taken back, come
   late: it is left. Each peer checks the other's Reveal against its commitment, and stops on one that doesn't
   match; then reads the other's side and checks it as it checks its own player's (`Settings::checked`: a
   build's check of the side as played). A side that breaks a rule ends both lobbies, each with the problems
   typed: `Reason::TheirBuild { problems }` on the peer that read it, and `Reason::YourBuild { problems }` on the
   one that sent it (its own check's answer of its own build; the refusal carries words for a log, not the
   problems). The seed comes from both nonces (SplitMix64), and the match is the agreed rounds, each part they
   leave picked from the seed alike on both, with both sides as played (`netplay::netplay_setup`). From here each peer has both sides, as the shared simulation needs them:
   `closed_builds` is not secrecy between the two programs, it is what each host shows its player
   (`Agreement::closed_builds`: the player's own build only, where with open builds a host may show both). (The
   commitment keeps a peer from choosing its side or its nonce after seeing the other's. It doesn't keep a peer
   that has committed from taking the other's Reveal and then leaving: such a peer has seen a build it didn't
   play, having first fixed its own.)
4. **Loading.** The match is agreed (`Status::Loading`, with the `Agreement`: the settings, the seed, both sides,
   the set and its match with every place stated, both names), and each host loads what it will draw and play of
   it. Each peer says how far its host has got, when it changes and every 100 ms: the match's seed, the digest of
   the match's first round at its start (below), a part of 255 and whether it is done
   (`Lobby::set_loading(fraction)`, `set_loaded()`), which the other host shows (`their_loading()`, 0 to 1;
   `their_loaded()`). The match starts (`Status::Start`) once this peer has loaded and
   has heard that the other has: by its loading message, or by a frame of the match, which a peer sends only once
   it has started. So neither runs a tick before both have loaded, and a peer whose last loading message is lost
   still starts, on the other's first frame. (The two start a moment apart, as they did when each started on its
   own agreement; clock sync levels them, §4.3.) One peer can take the match for agreed when the other doesn't:
   the other player took their ready back while the Reveals crossed, so its lobby went on to another revision
   before this peer's Reveal came. When this peer hears that lobby again it goes back to the lobby with it (`poll`
   says `Pending` after `Loading`), its player not ready, and what its host had loaded is for nothing. A host must
   go on polling its link and its lobby while it loads,
   every frame: load on another thread, or a little a frame. A link nobody polls for 3 seconds reads as a dropped
   connection on the other side (§4.8).
5. **The match, and after it.** The host takes the match's channel (`Lobby::channel`, once a match) and makes its
   `NetPlayer` on it; `poll` says `Playing` from then. The lobby hands the channel the frames of this match that
   the host gives `receive`, and puts the frames its player sends among `outgoing`'s. Through the match the peer
   goes on saying that it has loaded, every 100 ms: a peer that missed it hears it, and a peer whose host shows a
   result and ticks no player is still heard on the link (which takes 3 seconds of silence for a drop, §4.8).

   When the match is over, or has stopped with the link still up, the host drops the player and calls
   `Lobby::reopen`: the same two players are in the lobby again (the user's decision, docs/design/app-ui.md "After
   a match": "both players are back in the lobby on the same link, neither ready, with the same settings"; another
   match "needs no new invite code"). The settings and their stamp are as they were, nobody is ready, each may
   bring another build (`set_side`), and what they agree next is a new Hello and Reveal with a new nonce, so a
   match of another seed. Three things keep the match that was played out of the next lobby:
   - **The lobby's revisions count on** from where they were. What either peer said before the match (its lobby
     state, ready; its Hello; its Reveal, which says the revisions it is of), come late, is older than what it
     says now, and is left.
   - **A frame says its match.** One of the match before is none of the next lobby's (it doesn't mean the other
     has started), none of its loading's and none of the next match's player's.
   - **The other's ready mark is cleared** when a peer reopens: it was for the match that was played.

   Each player comes back when their host says, so **one may be in the lobby while the other still looks at the
   result**. The peer still in the match says nothing of its lobby, so its player is simply not ready yet; it
   takes nothing of the other's lobby meanwhile (settings changed there are adopted when it comes back, by their
   stamp, within 100 ms), and `Lobby::their_reopened` says the other is waiting. The peer in the lobby ignores
   the frames the other's player goes on sending. It may have gone before the other's match ended: its own set
   was over, and the other still lacked its last inputs (or the word that its player left the match). So a peer
   back in the lobby **says its player's last frame again** with its lobby's messages, every 100 ms, until it
   hears the other's lobby, which a peer says only once its match has ended. That frame holds every input the
   other hadn't acknowledged (§4.6), so it ends the other's match whatever was lost before it. (The frontend's
   test `the_player_back_in_the_lobby_first_leaves_the_other_its_last_frames` loses everything else the first
   peer had sent. With the frame not said again, tried by hand in that test, the other's player waited ten
   seconds and gave the connection up.) A `NetPlayer` whose set is over doesn't give the other up for silence, and one dropped then, on a lobby's
   channel, says nothing of leaving (`Channel::outlives_match`); one dropped before its set is over says it
   leaves the match (`MatchEnd`, §4.6), and the other's player stops with `StopReason::OpponentLeft`, the link
   and the lobby still there for both.

**Two builds of the program** must not get as far as a tick. Three things keep them from it, each ending both
lobbies with `Reason::DifferentVersion`:

- The protocol's version (`protocol::VERSION`, 1) stands for the wire's layout and for how a match is set up. It is
  the first field of every lobby message, Hello and refusal, and a peer of another version is refused on its first
  word. A test gives a lobby a peer's messages of another version, whatever follows that field, and it refuses
  each at once.
- The engine's version (`lobby::ENGINE`) is compared too, but it is the crates' version, which doesn't change when
  the engine does: it tells nothing apart today.
- So each peer also says, with its loading, the digest of the agreed match's first round at its start, as its own
  engine sets it up, and stops on one that differs from its own. That catches what a missed version would let
  through, where it shows in the round's setup or in the state's layout (both are in the digest), before the first
  tick and not at the first settled tick that differs. It can't catch two engines that start alike and step
  differently: the version is what tells those apart.

**A refusal** ends a lobby: the compatibility fields, a reason (a byte) and, for a log, the detail. A peer that
can't play with the other sends it three times and stops; the other stops on reading it. Each side ends with a
`lobby::Failure`: a `Reason` its host can say to its player, and the detail (§4.9). A player who leaves says so the
same way (`Lobby::leave`), from the lobby, from a match or from its result, so the other is told at once instead of
watching the link go quiet; that is the end of both lobbies, and the host drops its link. A Hello or a Reveal that
comes again, into the match, is answered with this peer's Reveal (the lobby does, with `Agreement::answer`).

**What a host does** with a link and a lobby is one loop, the same whatever its screen shows (the frontend's tests
`an_invite_becomes_a_match` and `two_matches_in_a_row_over_one_link` run it over WebRTC links, as `Seat::frame`).
It makes the lobby once the link is open (`Link::is_open`: the other player has come; until then a host waits on
its invite as long as its player likes), in the link's role, with a timeout longer than the link's 30 seconds of
reconnecting, so that a drop the link mends doesn't end the lobby. Then, each frame:

1. Take every datagram the link has and give each to `Lobby::receive`, a match's frames too. An error here is the
   link's, and the end (§4.9): the host sees it itself, in the lobby and in a match alike. Tell the lobby how the
   link is (`Lobby::set_link_down`, from `Link::down_for` and `gives_up_in`), which the match's player waits on
   and says (`NetStatus::reconnecting`).
2. If a match's player is up, tick it.
3. `Lobby::poll`:
   - `Pending`: show the lobby (`settings`, `ready`, `their_ready`, `their_name`, `their_present`, `ping`).
   - `Loading`: show the loading; step the match's `preload::Preload` (docs/frontend.md §9) and say how far it is,
     `set_loading(preload.fraction())`, then `set_loaded()`.
   - `Start`: take `Lobby::channel` and make the `NetPlayer` of the agreement's set on it.
   - `Playing`: the match, or its result. When it is over, or stopped with the link up, drop the player and call
     `Lobby::reopen`: the next poll says `Pending`. If `their_reopened` says the other is back in the lobby and
     this player's set isn't over, the other ended the match on its side: end it here the same way.
   - `Failed`: the end, with the reason to tell the player.
4. Send what `Lobby::outgoing` has: the lobby's own, and the frame the player made in step 2 (so it goes in the
   host's frame it was made in); also after a failure (a refusal) and after `leave`.

### 4.8 WebRTC: rooms, direct connect, reconnection

nettai-rtc carries the frames on a WebRTC data channel, as Tango does: one API on two backends, native (the `rtc`
crate, webrtc-rs's sans-I/O WebRTC) and the browser's (`RTCPeerConnection` through web-sys, on wasm32). It depends
on no engine crate. Two layers:

- `PeerConnection`: one connection and its data channel. It is told the other side's description and candidates
  and hands over its own as events (`PeerEvent`: the description, each candidate as it is found, open, down), so
  the browser's asynchronous offer and answer fit the same API; `send` and `recv` never wait.
- `Link`: the channel a match is played on, which keeps a connection up: it meets the other player (in an invite
  room of the signaling server, one it asks the server to make, `Link::create_room`, or one it has the code of,
  `Link::join_room`; or directly at a host's port, natively, `Link::host` and `Link::join`), and when the connection
  drops makes another (below), through the server's reconnect room or to the same address. Its `send` and `recv`
  are a datagram channel's; while it is down, what is sent is dropped and nothing comes.

**The channel** is opened unordered and without retransmits, and negotiated out of band (stream 0 on both sides, no
in-band announcement), so both sides make it alike and a lost datagram is the next one's to make up, never SCTP's to
resend late. WebRTC's framing adds about 94 bytes to each frame (§4.6). The link has a few messages of its own on the
same channel (the reconnect code and its acknowledgment, below). They are the channel's text messages, and the
game's datagrams are its binary ones: a data channel message says which it is (SCTP's payload protocol id), so the
two are told apart at no cost, and a frame has no byte more than before. `Link::send` and `Link::recv` carry only
the binary ones.

**The native backend** runs on the caller's thread and never waits: each call pumps the connection (the datagrams
that came, `rtc`'s timers when due, what it has to send, its events), and the frontend calls in every frame, often
enough for ICE, DTLS and SCTP. `rtc` gathers nothing, so a connection binds a UDP socket on each of the machine's
addresses (IPv4, and IPv6's global ones; the loopback address where there is no other) and offers each as a host
candidate; on the socket the system routes to the STUN server it asks for its address as the Internet sees it (a
server-reflexive candidate), and with a TURN server it allocates a relayed address (`rtc`'s TURN client, sans-I/O
too: what is sent from the relayed address goes through it, with a permission for each remote candidate). A
browser's candidates under an mDNS name (`.local`) are skipped; the browser still reaches the native side's host
or server-reflexive candidates, and the native side learns the browser's address from its checks. It relays over
UDP only: of a server's URLs it takes `stun:` and the first `turn:` without `transport=tcp`, and leaves `turns:`
(TLS) and TURN over TCP out (the browser takes them all). What may block runs on threads of its own: the
signaling WebSocket (tungstenite, rustls), and resolving the names of the STUN and TURN servers a welcome gives
(below). `Config::relay_only` offers relayed candidates alone, as the browser's `iceTransportPolicy: "relay"`
does (the tests' relayed connection). (`rtc` 0.21 passes a negotiated channel's open on only with the next datagram that comes, up to two
seconds later at the ICE keepalive; the backend hands it an empty one, which it drops, to drain it at once. ICE
checks a pair for as long as a dial is given, not `rtc`'s 1.4 seconds, so a dial made during an outage connects as
soon as the network is back.)

**The signaling server** is `signaling/`, a Cloudflare Worker in TypeScript with a Durable Object a room, on the
WebSocket hibernation API (an idle room costs nothing; `ping` is answered `pong` without waking it). TypeScript
because the Durable Object WebSocket API is the Workers runtime's own, with no build step under `wrangler dev`;
workers-rs would add a wasm build for a relay of about 300 lines. This is version 1 of its protocol (nettai-rtc's
`signal` module; a client says its version as it connects, `v=1`, and a server of another refuses it).

**The default server.** The user: "default signaling server should be matchmaking.nettai.n1gp.net". The app
meets through `wss://matchmaking.nettai.n1gp.net` (`DEFAULT_SIGNAL`, where the app builds its `Net`, in
`crates/nettai/src/screens/play/session.rs`), unless `$NETTAI_SIGNAL` names another (`ws://127.0.0.1:8787` for
`wrangler dev`). The Worker serves that hostname as a custom domain of its own, which its owner gives it
(`signaling/wrangler.toml` says how). No test reaches it: every test that makes a room's link names its server
(the in-process one, or `$NETTAI_TEST_SIGNAL`), and the app's tour pass its rig's.

**STUN and TURN are the server's to give.** The user: "add cloudflare turn support to signaling server", and "the
signaling server should return stun and turn server credentials"; and of the Google STUN server the clients had
by default, "you don't even need a google default right? direct connect doesn't even need stun?" So:

- Every welcome, in an invite room and a reconnect room alike, carries `iceServers`: the STUN and TURN servers
  the client's connections use, as the browser's `RTCIceServer`s (`{"urls":[...],"username":...,
  "credential":...}`, nettai-rtc's `IceServer`). With its TURN key, the Worker asks Cloudflare's TURN service
  (Cloudflare Realtime) for credentials for that client, `POST
  https://rtc.live.cloudflare.com/v1/turn/keys/{TURN_KEY_ID}/credentials/generate-ice-servers` with
  `Authorization: Bearer {TURN_KEY_API_TOKEN}` and `{"ttl":14400}`, and gives the servers it answers: its STUN
  server (`stun:stun.cloudflare.com:3478`) and its TURN servers (`turn:` over UDP and TCP, `turns:` over TLS,
  each with the username and credential). Without the key, or when the service answers with an error, gives no
  server, or doesn't answer within 3 seconds, the welcome gives Cloudflare's public STUN server alone (a constant
  in the Worker): a room never waits on the service longer, nor fails for it. The failure is logged on the
  Worker, by its kind or HTTP status; the secrets are sent to Cloudflare's API and nowhere else, and never
  logged.
- **TURN only for a room both players are in, and only so often.** The credentials cost the key's owner what is
  relayed on them, and anyone could open a socket to `/rooms` (the security audit's finding; the user: "fix the
  open turn credentials as well"). So a welcome gives TURN credentials only once the room's two places are taken;
  before, STUN alone. The client that waited alone (the host of an invite, the first to a reconnect room) is given
  its TURN credentials as the other comes, in the peer message (`{"type":"peer","present":true,"iceServers":[...]}`),
  which a client takes as a welcome's servers; a `peer` without them leaves the servers as they were. And one
  client, by its IP address (Cloudflare's `CF-Connecting-IP`; an IPv6 address by its /64), is given credentials at
  most 10 times a minute (the Worker's rate limit binding `TURN_LIMIT`, in `wrangler.toml`; a pairing is two, both
  counted against the client whose coming made it): past that, STUN alone, logged. The in-process test server gives
  its servers the same way, so the room tests take the host's from the peer message.
- The credentials last four hours (`signal::TURN_CREDENTIALS_LAST`, the Worker's `TURN_TTL_SECONDS`): a long
  sitting of matches with one opponent, and its reconnections. Every reconnection goes through a reconnect room,
  whose welcome gives new ones; a relayed connection that outlasts its credentials may drop when its allocation
  can't be refreshed, and comes back the same way.
- A client uses exactly the servers the latest welcome gave, for each connection made after it (so a
  reconnection has its reconnect room's). `Config::ice_servers`, empty by default, is a client's own servers,
  used only where a server's welcome gives none (the in-process test server, unless a test sets some) and by a
  `PeerConnection` made by hand. A direct link uses no STUN or TURN server: it says no candidate to anyone (the
  joiner dials the host's address, and the host learns the joiner's from its checks), so its address as the
  Internet sees it would go nowhere, and a relay would need a server to hand out credentials.
- No connection is made on servers it hasn't had. The server sends the welcome before anything of the other
  client's, and a joiner dials only once it is in the room with the other (which the welcome, or a `peer` after
  it, says). The welcome's names are resolved natively on a thread of their own (the browser resolves its own),
  and until they are (at most 5 seconds; then the servers from before), the joiner doesn't dial and a host holds
  the joiner's offer, and the candidates that follow it, before answering. A room's link waits on nothing as it
  is made.
- **The owner's setup.** Make a TURN key (the Cloudflare dashboard's Realtime, TURN Server, Create; or the API's
  `POST /accounts/{account_id}/calls/turn_keys`): it shows the key's ID (the "Turn Token ID") and its API token.
  Give them to the Worker as its two secrets, `npx wrangler secret put TURN_KEY_ID` and `npx wrangler secret put
  TURN_KEY_API_TOKEN` in `signaling/` (for `wrangler dev`, the same names in `signaling/.dev.vars`, which git
  ignores); then `npx wrangler deploy`. Neither secret is ever in the repository. Deploying, the secrets and the
  custom domain are the owner's steps, never an agent's.

**The server only while it is needed.** The user's request: "the signaling server session should last just long
enough for the peers to reach each other, so it doesn't have to hold onto a connection longer than required. but it
should also support reconnect somehow, maybe by some special lobby code that the two players agree on for each
reconnect?" So the two players are in a room of the server only until their connection is through, and again while
a dropped one is made anew. Between those times neither has a socket to it: a lobby, a match and the lobby after
it (another match with the same opponent) need no server. There are two kinds of room:

- **An invite room** is where two players first meet. Its code is the server's to pick, never a client's (the
  user's instruction: "signaling server should assign invite code instead of the client generating one"). It is six
  characters of `ABCDEFGHJKMNPQRSTUVWXYZ23456789`: capitals and digits without `0` and `O` and without `1`, `I` and
  `L`, so that none reads as another; some 887 million codes. The server draws it evenly from its own randomness
  (`crypto.getRandomValues`), and draws again if a room has it. A code as a player types it has its letters made
  capitals and its spaces and hyphens dropped (`signal::normalize_code`): a host may show it in two groups of three.
  To make one a client opens a WebSocket to `/rooms?peer=ID&v=1` (ID the client's own, random, the same on every
  reconnection of its socket). The server makes the room, the client is its host (side 0, the left navi), and the
  server's welcome says the code (`Link::code`). To join one, or to come back to one after the socket dropped, a
  client opens `/rooms/CODE?peer=ID&v=1`. A code no room has is refused as that, and no room is made of it.
- **A reconnect room** is where the two meet again after their connection dropped, by a code they agreed on over
  that connection (below): 32 lowercase hex digits, 128 random bits. Both open `/reconnect/CODE?peer=ID&v=1`. The
  first to come makes the room and the second joins it. A code not of that form is refused (`bad-request`). The
  server can't tell a code that was used before from a new one, and doesn't need to: the clients never go back to
  a spent one, and nobody else knows it.
- A room has two places. In an invite room its maker hosts and the next ID to come joins; in a reconnect room the
  places are only the first and the second to come, and the clients keep the roles they had. An ID that comes again
  gets its place back, which is how a client whose socket dropped keeps its side, and a third is refused.
- The server says `welcome` (in an invite room the place, whether the other is there, and the invite's code; in a
  reconnect room whether the other is there; in both, the STUN and TURN servers, below), `peer` (the other came or went), `signal` (what the other sent) and
  `error` (a refusal, after which the socket closes: its `reason` is `not-found`, `full`, `version`, `bad-request`
  or `busy`, and its `message` is for a log); a client says `signal` (relayed to the other, dropped if it isn't
  there) and `leave` (gives its place up).
- It reads none of the signals: the joiner's offer, the host's answer, the candidates, each of a connection's
  generation, and the host's call to dial again.
- A room both players left is forgotten at once, and its code with it. Otherwise a room is kept while a socket of it
  is open, and after the last one goes ten minutes for an invite room (a player whose socket dropped comes back to
  its place) and 30 seconds for a reconnect room (the link's time to reconnect, `signal::RECONNECT_ROOM_KEPT`);
  then it is forgotten. A code whose invite room is forgotten is refused like one that never was a room's.
- Taking a place and giving one up each read and write the room's places with nothing else between
  (`blockConcurrencyWhile`): the two leave a room at about the same moment, and come to a reconnect room at about
  the same moment, and each must see the other's doing.

**The code of the next reconnection** is agreed over each connection, once it is open, so the server first sees a
code when it is used. The host draws it from the system's cryptographic randomness (`getrandom`; the browser's
`crypto.getRandomValues`), 16 bytes, a new one for every connection, and says it (`code HEX`, a text message)
every 100 ms until the joiner acknowledges it (`ack HEX`), the way the lobby says its part again; the joiner
acknowledges every one that comes. The channel is DTLS's, so nobody between the two reads it.

- **Through.** The connection is through, for the joiner, when the code comes: it has heard the host on an open
  channel. For the host it is when the acknowledgment comes: a datagram has gone each way. Each side then gives its
  place in the room up (`leave`) and closes its socket. Once both have, the server forgets the room, and an invite's
  code names no room from then. Before that the link stays in the room it met in, as it did, and an invite waits as
  long as its room is kept.
- **A drop** sends each side to the reconnect room of the code agreed last; before any is agreed, to the invite
  room, which neither has then left. A code agreed is spent once the next one is: until then it is the one the two
  meet by (its room made again if it was forgotten), so a drop before the new code is acknowledged reconnects by
  the code from before. The joiner always knows its one code. The host doesn't, when its newest code isn't
  acknowledged: the code may have been lost on the way, or only the acknowledgment. So it waits in that code's room
  as well as the one from before, and keeps the room the joiner's offer comes through (the joiner has the new code
  if, and only if, it is there), leaving the other.
- Roles stay as they were in a reconnect room: the joiner dials and the host answers, and the generations go on
  counting. Once the new connection is through, both leave and close, a new code agreed for the next drop.
- A reconnect room's socket that doesn't open, or drops, is made again (0.5 seconds, then twice as long each time,
  up to 5): the server out of reach is no reason to stop before the link's 30 seconds are out, and then it is
  `ConnectionLost` as for any drop that didn't come back. Meanwhile `Link::problem` says the server can't be
  reached. A refusal in a reconnect room (a third socket in it, or another version) ends the link: the connection
  is lost (`ServerVersion` for the version).

nettai-rtc's `testing` feature has a server of its own speaking the protocol, in process, for tests (it picks codes,
refuses and forgets as the Worker does; `Server::forget` ends a room at once, `Server::set_reachable` takes it off
the network, `Server::set_ice_servers` sets the servers its welcomes give, none at first, and `testing::Client`
is a socket of a test's own, for either server). The link tests check that a room's connections ask the
welcome's servers and no others (STUN and TURN with credentials, STUN alone without; the links' own only when a
welcome gives none; a direct link none), and connect relayed alone through a TURN server of the tests' own
(`tests/turn`: long-term credentials, allocations, permissions, indications and channels on loopback, a few
hundred lines on `rtc`'s STUN and TURN types), which refuses other credentials. The Worker is run locally
(`npx wrangler dev`), and nettai-rtc's link tests run against it too, with `NETTAI_TEST_SIGNAL=ws://127.0.0.1:8787`
(those that look inside the server, or take it off the network, run on the in-process one only); deploying it is
its owner's step (`npx wrangler deploy`).

**Direct connect** (native only) exchanges nothing before ICE: each side makes up the other's description, every
field of which is fixed but two. The ICE credentials are fixed (the joiner's username fragment is its nonce, drawn
once a link, and the connection's generation); the joiner is told the host's address, its one remote candidate,
and the host learns the joiner's from its first connectivity check (a peer-reflexive candidate). Neither side's own
candidate is said to anyone, so each is a placeholder naming its socket, bound on every interface (`0.0.0.0:PORT`).
The DTLS fingerprint is the one field a real exchange carries that can't be made up, and it isn't checked: direct
connect authenticates nobody, as the raw UDP it replaced didn't (a room's connection checks the fingerprints its
descriptions carry). It crosses a NAT as that did: on a LAN, or with the host's UDP port forwarded to it.

**Reconnection.** The joiner dials (the offer, ICE's controlling side, the DTLS client) and the host answers, in a
room and directly alike. A dropped connection isn't mended (no ICE restart): the joiner dials a new one, the next
generation, which works the same on both backends and directly; what is said of an older generation is dropped. A
drop is the connection's state (disconnected, failed or closed), its data channel closing, or silence: nothing from
the other side for 3 seconds while it is open (the lobby sends every 100 ms, a match every frame). Then the joiner
dials again at once, and again after 0.5, 1, 2, then every 3 seconds while dials don't open (each given 10
seconds), and in a room also when the host says it lost the connection, or when the host comes into the room. The
host waits for the next generation: its offer in the reconnect room (asking for it every 2 seconds while it has no
new connection under way), or directly the joiner's checks, whose username fragment names a newer generation (the host
reads its socket itself, and takes only the joiner whose nonce came first). After 30 seconds down the link gives
up, and every call after errs (`ErrorKind::ConnectionLost`; its detail: "the connection to the other player dropped
(WHY) and didn't come back within 30 seconds"); while it is down, `Link::down_for` says for how long and
`Link::gives_up_in` how long is left. What a connection had taken before it dropped is still handed over
(`Link::recv` gives it first): a player who leaves says so in their last datagrams, which come with the close that
follows them. The connection doesn't need the server while it is up, and has no socket to it then.

**Before the first connection** nothing is down, and two waits end the link instead. The signaling server not
letting the player into the invite room within 10 seconds (`Config::server_timeout`), or its socket not opening at
all, is `ErrorKind::ServerUnreachable`, at once: a player who asked for a room is told now, and nothing is tried
again (once the server has let the player in, a socket that drops is made again for as long as the room is kept). And
once the other player is there to connect to (in the room; dialed directly; dialing a direct host) with no
connection open after 30 seconds (`Config::connect_timeout`), the link gives up with `ErrorKind::CantConnect`: no
path between the two networks was found, or nothing answers at the address. How long a host waits for the other
player to come at all is its player's to say: an invite waits as long as its room is kept.

The match needs nothing of its own to carry on: while the channel is down both peers wait at the stall guard (30
ticks of unconfirmed input, about half a second after the drop), still sending their windows, which are lost; the
gap can't pass twice the stall guard, inside the horizon (§4.6); when it is back, the next datagrams carry what each
side hasn't acknowledged, rollback corrects what was predicted, and clock sync levels the peers again. What the
frontend does is not give up: `NetPlayer` ends a match after 10 seconds with nothing from the other player, but not
while the channel says it is down (`Channel::reconnecting`), counting again from when it is back, and not once its
set is over; giving up is the transport's. The players see the battle stop within half a second; a host says that it is reconnecting, and
counts down what the link has left (`NetStatus::reconnecting`: how long it has been down, and
`gives_up_in`, §4.11); the battle goes on where it was, or the match ends after 30 seconds with the error above.

### 4.9 What a player is told when it goes wrong

The user's instruction: "don't show any internal error strings to the user. if there are internal errors that must
be surfaced as user errors, they should be replaced with enums and then go through i18n. they should not be overly
technical errors". So nothing in the netplay path hands a host a sentence to show. Each layer's error is a kind (an
enum) and a detail: the host says the kind in its own words, from its catalog, in the player's language, and writes
the detail (English, as specific as the code can make it) to its log.

**nettai-rtc**: `Error { kind: ErrorKind, detail: String }`, from `Link`'s constructors and from every call on a
link that has ended (the first error a link meets is the one every later call returns).

| `ErrorKind` | When |
|---|---|
| `ServerUnreachable` | The signaling server's socket didn't open, or the server didn't let the player into the invite room within `Config::server_timeout` (while a dropped connection is made again, the server out of reach is only what `Link::problem` says, and the link goes on trying until it is `ConnectionLost`) |
| `ServerVersion` | The signaling server speaks another version of its protocol than the program: one of the two is out of date |
| `NoSuchRoom` | No room has that code: mistyped (refused before the server is asked, if it can't be a code), or its room is over |
| `RoomFull` | The room has two players already |
| `BadAddress` | A direct host's address names no machine |
| `PortUnavailable` | A direct host can't listen on its port |
| `CantConnect` | The other player is there and no connection could be made within `Config::connect_timeout` |
| `ConnectionLost` | The connection dropped and didn't come back within `Config::reconnect_timeout` |
| `Internal` | Anything else (no socket, no cryptography, a server that broke its protocol): nothing the player did or can mend |

**The lobby** (nettai-frontend): `lobby::Failure { reason: Reason, detail: String }`, from `Status::Failed`. A
refusal crosses the wire as a reason byte and the detail, and each side has its own reason for it: the peer that
finds the other's build broken has `TheirBuild`, and the other, refused, has `YourBuild`.

| `Reason` | When |
|---|---|
| `DifferentVersion` | The two programs differ: another netplay protocol, another engine, or engines that set the agreed match up differently (the digests of its start differ: §4.7). (A refusal from a peer of another protocol, in whatever form that protocol has, reads as this.) |
| `DifferentGame { theirs }` | The other player's match is of another game, `theirs` by its id |
| `DifferentContent` | The same game, but other content: the two players' packs differ |
| `YourBuild { problems }` | This player's build breaks a rule of the game as the match would play it, by the other side's checks, which refused it. `problems` is this peer's own check of it, typed (none where this peer finds nothing wrong: the two programs differ). A host never gets here by its own player's doing: a build with problems can't be readied (`Lobby::problems`, `Lobby::set_ready`) |
| `TheirBuild { problems }` | The other player's build breaks a rule of the game as the match would play it (their Reveal's side didn't pass a build's check): `problems` is what, typed |
| `OpponentLeft` | The other player left the lobby |
| `Left` | This player left the lobby (`Lobby::leave`) |
| `NoAnswer` | Nothing came from the other side for the lobby's timeout |
| `Protocol` | The other side didn't keep the handshake (a Reveal that isn't what it committed to, settings that don't read, the same role), or says this side didn't: nothing a player can mend |

**The match** (nettai-frontend): `driver::StopReason`, from `Player::stop_reason` (`Player::stopped` is the detail).

| `StopReason` | When |
|---|---|
| `Finished` | What was played came to its end: the set was decided, or a recording's input ran out |
| `OpponentLeft` | The other player left the match before its end (their player was dropped). On a lobby's channel both are still in the lobby, and a host goes back to it (`Lobby::reopen`); a player who leaves for good says so in the lobby (`Reason::OpponentLeft`). On a channel that is the match's alone, a player dropped after the match's end reads as this too (the result says which) |
| `ConnectionLost` | Nothing came from the other player for 10 seconds while the link was up and the set not over; or a host's own channel gave up (a lobby's channel has no errors: the link's `ConnectionLost` is the host's to see, from the link) |
| `Internal` | The engine stopped, or the other side's datagrams stopped making sense (not the protocol's, or input past the rollback horizon) |

The app's first list of messages (docs/design/app-ui.md §7) is made of these: "Can't reach the server" is
`ServerUnreachable`; "No match with that code" is `NoSuchRoom`; "That match is full" is `RoomFull`; "Your opponent
has a different version of nettai" is `DifferentVersion` (and `ServerVersion`, which is the program's own being out
of date) and `DifferentContent`; "This build can't be used" is `YourBuild` (what breaks is the build's own check
to say, before the lobby: `nettai_match::check_side`); "Connection lost. The match has ended." is `ConnectionLost`
of either layer; and "Something went wrong" is `Internal` and `Protocol`. The rest have no message in that list yet
(`DifferentGame`, `TheirBuild`, `OpponentLeft`, `NoAnswer`, `CantConnect`, `BadAddress`, `PortUnavailable`).

### 4.10 The input delay

The user's instruction: "input delay should be configurable and not fixed and default to 4". It is each player's
own setting, in ticks: `NetOptions::input_delay`, `netplay::DEFAULT_INPUT_DELAY` (4), `netplay::INPUT_DELAY` (the
range a host offers, 0 to 10; a value outside it is brought inside), `Player::set_input_delay` during a match, and
`NetStatus::input_delay` to show.

**It is getgud's present delay**, and that is what "input delay" means in a rollback fighter. Two things were
candidates, and they turn out to be one:

- The frontend's "present delay" (`Player::set_present_delay` until now, default 0): the peer shows the frame
  that many ticks behind the player's newest input.
- A delay under it, in the rollback layer, that would hold a press back before it is simulated or sent, as GGPO's
  "frame delay" is usually described. getgud has no such second knob: its one knob is the present delay, which its
  own documentation calls "the classic input-delay knob". nettai-netplay's own session, before getgud, called the
  same thing `input_delay` (§4.4), and the engine has no link delay of its own (§1.1).

They are the same delay counted from either end. A fighter with a frame delay of `d` schedules a press for the
frame `d` ahead and shows the present frame. getgud takes the press for the newest tick and shows the tick `d`
behind it. Either way the player sees their press take effect `d` frames after they make it, and the other
player's input for the frame shown has had `d` frames longer to arrive: a one-way trip of up to `d` ticks needs no
prediction, so nothing is rolled back. What differs is only which tick is called "now". In both, the press goes
to the other peer in the frame it is made, whatever the delay.

So the setting is the present delay under the player's name for it, and there is nothing else to set. It never
crosses the wire and no tick depends on it: both peers settle the same ticks from the same inputs whatever each
one's delay is (the frontend's set test plays one player at 3 and the other at 0, changed mid-round, and the
settled states agree). 4 is also the delay of the original's own link queue (4 ticks in the recordings, §1.1), so
a match at the default answers a press when the original's netbattles did; and it hides a round trip of some
130 ms (§4.11's "good"). The range ends at 10 ticks, a sixth of a second, past which a navi answers too late
to play; 0 shows every tick as soon as it can be, all of the other player's late input predicted.

### 4.11 Connection quality

The user's instruction: "in play you should show connection quality". A host's indicator of four bars needs a
level; the level needs two figures, both in milliseconds (`nettai_frontend::connection`):

- **The ping**: the round trip of a message to the other player and back, smoothed.
- **The jitter**: how unsteady it is, the smoothed distance of each round trip from the ping.

They are RFC 6298's SRTT and RTTVAR: each new round trip moves the ping an eighth of the way to it and the jitter
a quarter of the way to its distance from the ping; the first round trip is the ping, and half of it the jitter.

**Where they are measured.** In a match, by the input link (nettai-netplay's `LinkStats::srtt` and `rttvar`): each
datagram's newest element is timed until an ack covers it, 60 times a second. That round trip includes the wait
for the frame that takes the datagram on each side, a tick or so in all, which is right for a figure about how
late the other player's input is. In the lobby, by the lobby's own messages (`Lobby::ping`): each carries its
sender's clock, and the other's latest clock said back with how long it was held, so a peer times the trip without
the 100 ms between messages in it.

**The quality** (`connection::Quality::of(ping, jitter)`, and `NetStatus::quality`) goes by the ping plus twice
the jitter, a round trip few are slower than. What a rollback match feels is the other player's input arriving
late, by half the round trip; an input later than the player's input delay is predicted, and a wrong prediction is
a rollback. So the thresholds are ticks each way (a tick is 16.7 ms):

| `Quality` | Bars | Ping + 2 × jitter | Each way |
|---|---|---|---|
| `Excellent` | 4 | up to 70 ms | about 2 ticks |
| `Good` | 3 | up to 135 ms | about 4 ticks: the default input delay hides it |
| `Fair` | 2 | up to 200 ms | about 6 ticks: rollbacks of a tick or two at the default input delay |
| `Poor` | 1 | more | more |

The loss is a figure of its own (`NetStatus::loss`) and doesn't move the level: a lost datagram's inputs come with
the next one, so loss costs a frame per datagram lost in a row (§5.1), and shows as jitter where it matters.

**While the connection is down** there is no quality to show. `NetStatus::reconnecting` is then
`Reconnecting { elapsed, gives_up_in }`: how long it has been down, and how long the transport goes on trying
(nettai-rtc's `Link::gives_up_in`, of its 30 seconds), which is what a "reconnecting" countdown shows. In the lobby
the link says the same itself (`Link::down_for`, `Link::gives_up_in`).

## 5. Results

### 5.1 Synthetic netbattles (in this repository)

`crates/nettai-netplay/tests/rollback.rs`: two navis with 300 HP on the battle settings 0 of the engine's test content
(`content::testing`: EXE6's modules the tests need and made-up chips on the engine's own actions, not EXE6's data):
side 0's folder holds a level-3 GunDelSol, an eraser navi chip, a level-1 GunDelSol, an invisibility dimming chip
and a level-3 GunDelSol over and over, side 1's GunDelSols only (no Crosses or Beast Out). Both players mash (held
buttons change every four frames on average: a direction, A, B, L or R; START is never pressed), and the mashing
drives their custom screens too. Three seeds, each under every configuration, over getgud sessions whose
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

Every battle runs to the end with every settled state either peer returns equal to the other's and to the lockstep
run, and ends the same way as without rollback; each peer's sound plays every confirmed cue once. (The frames are
where both peers' settled states are over, give or take the one frame a peer settles past the other.) Rollbacks come
at about the same rate per frame as over the ordered link this replaced (a fifth of the frames at 10 + 3, the
deepest 13), though the battles are longer now (the test content changed since). A lost datagram's inputs come with
the next one, so loss costs a frame per datagram lost in a row: the deepest rollback grows by about the longest run,
and the peers stall a little more for clock sync. getgud rolls back less often than the peer nettai-netplay had
before it: it checks predictions as rows settle, promotes the prefix that held, and catches up on several arrivals in
one rollback.

The other tests: the engine's own input record with the events riding in player 0's input (latencies 3 and
8, in sync), observers seeing every simulated, rewound and settled frame and nothing past the end of the input,
clock sync (a peer that starts 6 or 20 frames ahead), the two negative tests below, and:

- **ticks that panic** (§7.1): on predicted input only (a game whose prediction after L is an input no player
  sends), rolled back, the match as without it; on confirmed input, both peers and the lockstep run stopping on
  the same tick, in sync;

- **an outage**: nothing gets through either way for two seconds (120 frames, at 3 + 1): both peers wait at their
  stall guard (114 frames each), the match goes on after it, in sync, to the KO;
- **a gap past the horizon**: with a horizon of 32 under a stall guard of 40 (one that doesn't fit, §4.6), one
  direction fails for five seconds; the peer that hears nothing waits at its guard while the other runs on to its
  own, and when the link comes back the gap is wider than the horizon: that peer tears the match down at tick 300,
  where both settled states agree. With the full horizon, the same match goes on;
- **a set's rounds**: two `Peer`s over a lossy network (4 + 2, 10% lost, 5% duplicated) play a best-of-three set
  as a frontend does, ending each round when their settled state is over and starting the next on the same stream:
  the settled states agree in every round, and when the set is over one player leaves and the other hears it.

An ignored test mashes with everything the test content has on both sides (cut-ins, giga cut-in chips, navi chips,
bombs, swords, traps, grabs, 500 HP): no tick stops, settled or speculated (§7.1).

**Over a channel, and over WebRTC**: the frontend's tests play two `NetPlayer`s on EXE6's content over a channel the
test shuttles by hand, each frame arriving three frames after it was sent (the agreement from both offers, each
player's own loadout, 900 ticks of mashing; and a set to its end, in both games), and their settled states agree.
Over WebRTC links in a room of the in-process signaling server, a short set (the host's navi shoots, the joiner's
has 1 HP) is cut mid-round-one by three seconds of the joiner's network going: both see the channel down, a new
connection comes back about 0.1 s after the network does, and the set goes on to its end, both replays the same but
for who recorded, one playing back with no difference. nettai-rtc's tests open links on loopback (directly in about
0.1 s, through a room in about 0.15 s), see both leave the room once through (the server keeping nothing, the
invite's code then naming no room), bring them back after outages of either side (one generation an outage) through
the reconnect room of the code agreed before it, each drop by the next code, also when the server's sockets drop
in the middle, and when the newest code or only its acknowledgment was lost before the drop, give up after the
reconnection timeout (also with the server out of reach meanwhile), and are refused, each by its kind, where they
should be: a third player in a room or a reconnect room, a reconnect code not of the clients' form, a code no room
has (one that can't be a code, one that was never a room's, one whose room both players left, one whose room's time
ran out), a server that isn't there or doesn't answer or speaks another version, a direct address that names
nothing, a port that is taken, a host that never answers; a stranger in the room of a spent code meets nobody. The
same tests pass against the Worker under `wrangler dev` (but those that need a room's time to run out or the server
off the network, and the checks that look inside the server), and in a browser (headless Chrome) two connections in a page, two links in a room, and
a browser's link and a native one in the same room trade datagrams. The frontend's tests over links in a room go
the way a Play screen does: `an_invite_becomes_a_match` (an invite's code typed, the lobby with its settings
changed, each side's preload feeding the loading, the match on the lobby's channel, one player leaving the match and
then the lobby) and `two_matches_in_a_row_over_one_link` (a single battle played to its end; the host back in the
lobby for a second and a half while the joiner's player still runs, its frames none of the lobby's; another build;
a second match of another seed with that build, a frame of the first coming late into its loading and into the
match; both back, and one leaving). nettai's `netplay` test agrees a match in a
room of an in-process server, both sides polled on one thread, and plays it over the links (nettai-demo's `net`
tests, retired with it, did the same directly and in a room). (Earlier, over plain UDP, two
frontends in their windows on loopback played with an 18 ms round trip, the joiner seeing the battle from its side.)

### 5.2 How long before a divergence, and why

With the engine as it is, never: no configuration diverged. Two deliberate breaks show the checks work and what
would break it:

- **Peers simulating from their own side**: out of sync from the intro's first frames (the fade-in, §2).
- **State outside the snapshot** (a counter shared between a game and its snapshots, as an `Rc` or a static
  would be): out of sync at tick 37 at 4 frames of latency, on the first rollback that re-simulated a frame
  reading it.

### 5.3 The golden traces under rollback (verification workspace)

The golden-trace suite outside this repository replays each round's recorded inputs through two getgud sessions
(both simulating the trace's side, the custom-screen events in player 0's input), their inputs carried by rennet
over the simulated datagram network, up to the frames the plain replay matches, where the input ends and the peers drain their sessions. Each round is played over eight
networks: latencies 0, 2 + 1, 5 + 2 and 10 + 3 as they are, and lossy (10% of the datagrams lost, a lost one
followed by another 30% of the time, 5% duplicated; at 10 + 3, 20%, 50% and 10%). Every frame either peer confirms
(its last simulation before it settles, which the world reports) and every settled state getgud returns with a
frame (most frames) must match the trace exactly, the peers must agree, and each peer's sound (its cue actions through EXE6's sound calls and the driver) must
be the plain replay's, later by at most the latency and the longest run of lost datagrams. All of them do:

| Round | Frames | 0 | 2 + 1 | 5 + 2 | 10 + 3 | 0, lossy | 2 + 1, lossy | 5 + 2, lossy | 10 + 3, lossy |
|---|---|---|---|---|---|---|---|---|---|
| machgun 1 | 1,074 | match | 78 | 81 | 79 (12) | 2 | 80 | 81 (10) | 73 (16) |
| machgun 2 | 1,331 | match | 62 | 61 | 61 (12) | 2 | 60 | 62 (8) | 57 (17) |
| soundmod 1 | 21,962 | match | 1,103 | 1,100 | 1,100 (13) | 51 | 1,076 | 1,096 (10) | 1,069 (18) |
| soundmod 2 | 14,933 | match | 878 | 880 | 877 (13) | 32 | 871 | 877 (10) | 845 (17) |
| soundmod 3 | 20,436 | match | 1,440 | 1,436 | 1,429 (13) | 64 | 1,426 | 1,433 (9) | 1,376 (17) |
| bn67-amogus 1 | 65,477 | match | 3,247 | 3,206 | 3,160 (13) | 121 | 3,169 | 3,171 (11) | 3,016 (18) |
| lmao-chonked 1 | 23,825 | match | 1,565 | 1,557 | 1,543 (13) | 66 | 1,540 | 1,527 (10) | 1,457 (18) |
| a 2022 round (legacy, the mixed one) | 34,707 | match | 2,297 | 2,273 | 2,250 (13) | 90 | 2,252 | 2,243 (11) | 2,152 (18) |

(Every frame of every round matches on both peers; the numbers are player 1's peer's rollbacks, which receives
player 0's buttons and the custom-screen events, with the deepest in brackets. bn67-amogus2, bn67-lilguy,
gregar-sitteruno and the 2022 guard-through-Cross round match likewise; the two longest 2022 rounds are run with
`--ignored`, and match too.) Loss without latency costs a few dozen shallow rollbacks a round (a lost datagram's
inputs come a frame late, with the next one); with latency it changes little, since the window resent every frame
already covers a loss: what it adds is the run of datagrams lost in a row (up to 15 here), which deepens the
deepest rollback (13 to 18 at 10 + 3) and makes clock sync stall more (soundmod round 1 at 10 + 3: 44 frames of
22,017 clean, about 155 of 22,166 lossy). The sound: at most 9 frames late at 10 + 3 clean and 13 lossy; a sound
played on a prediction and stopped again up to 53 times in a 65,000-frame round. A frame costs 6.2 to 6.6 bytes
without latency, 9 to 10 at 2 + 1, 16 at 5 + 2, 27 to 28 at 10 + 3. getgud returns a settled state with most
frames, and each is compared with the trace: at 10 + 3, soundmod round 1's peers compared 21,391 and 21,269 of its
21,962 frames' settled states (the rest were re-simulated without a save, and checked through their last
simulation).

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
same.) That is under 0.5% of the 16.7 ms a frame has at 60 fps. The digest dominates and is needed once per settle,
not per re-simulated frame.

A real session costs far less than the worst case, since most frames promote their prediction: soundmod round 1
through two sessions at 10 frames of latency (no jitter, the netcode alone) cost 5.7 to 6.1 µs per rendered frame
with getgud and 6.2 to 6.6 µs with the old peer, measured side by side.

(Not measured again for getgud's nettai-fixes branch (§4.5): the per-frame path gained a `catch_unwind` around
each tick, which costs nothing unless a tick panics, and getgud now keeps a call's promoted states until the next
call instead of recycling them at once.)

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
- **Speculative panics.** A peer simulating a predicted input explores input sequences no player made yet, and a
  tick that panicked on one crashed a peer that was otherwise in sync, inside `Session::advance`, between getgud's
  load and save. Now a peer steps its battle with `step_game`, which catches a panicking tick and stops the battle
  instead (`Battle::fail`): the round ends with `RoundEnd::Error` and the panic's message, the state stays as the
  failed tick left it, that tick makes no sound, and a stopped battle doesn't tick again. The failed tick ran the
  same code on the same state up to the panic on both peers, so the stop is part of the simulation like any
  other state: on predicted input it is rolled back with the speculation that reached it, and on confirmed input
  both peers (and the simulator's lockstep run) stop on the same tick in the same state, and the match ends
  there in sync (`a_tick_that_panics_on_predicted_input_is_rolled_back`,
  `a_tick_that_panics_on_confirmed_input_ends_the_match_on_both_peers`). The digest covers that the battle
  stopped but not the message (a content runtime's can quote addresses). getgud never sees an error, so its
  session is never left half-way; and getgud itself now stays consistent after a `World` error (§4.5).

  What panics remain: no path of the original is left unported (the old list, cut-ins, ElmntMan's elements,
  Beast Out's head and rush, the damage judge, is all in the engine). The engine still panics on content errors
  (a script's error, a role the ruleset needs left unfilled, a malformed definition), on states the original
  can't go on from (it would read past a table, through a null pointer, or loop forever: the engine says which
  and where), on what only a single-player battle does (the menus' restart after a communication error, battle
  modes netbattles don't have), and on bug codes naming stat bytes no hit of the game names. All of these now
  stop the battle alike on both peers. The synthetic matches mash with everything the test content has on both
  sides, dimming chips (so cut-ins and counter cut-ins), giga cut-in chips, navi chips, bombs, swords, traps and
  grabs, Crosses and Beast Out unlocked, and 500 HP, and no tick stops, settled or speculated
  (`mashed_battles_with_everything_never_stop`, ignored for its time; the test content's MegaMan has no Cross or
  Beast form, which the golden traces cover). It found one content error: the test content's roles lacked the
  chips the custom screen's Beast Out button and an invalid selection become, which stopped such a battle on
  both peers at once, as it should; the test content has them now.
- **Per-viewer visibility.** The blindness rules, the Beast Out lock-on marker and the A-charge glow decided
  `VISIBLE` for the local side, so the other player saw the local side's view: its own navi gone while the local
  player was blind, the other side's objects in plain sight while it was blind itself, and the local player's
  marker and glow instead of its own. Not cosmetic: blindness is a status a chip inflicts. Each such rule now
  decides the object's visibility for both viewers (§2.3: `Object::sight`, `Battle::visible_to`), the overlays
  and attachments that take their owner's visibility copy each viewer's, and the `VISIBLE` flag stays the local
  side's, so the golden traces and the whole chip lab still match it frame for frame.

### 7.2 Found, still open

- **Per-viewer looks**: a few looks still follow the local side (§2.3): the counter blink (the other side's navi
  blinks blue for a local player in Full Synchro while it can be countered), the barrier parts hidden unless the
  navi is the local MegaMan, and the trap mark's facing. A sprite has one `Look`, so these need a look per
  viewer; cosmetic, and the digest leaves looks out.

### 7.3 Checked, not a problem

- No floats, statics, thread-locals, `Rc`/`RefCell`/`Cell`, hash maps, clocks or I/O in the simulation (the
  frontend's thread-local panic flag and nettai-audio's floats are presentation; the content's `Arc` points at
  immutable data, and the per-thread runtime cache holds code, not state). A tick that panics unwinds through the
  content runtime cleanly: mlua turns a panic in a callback into a Luau error and resumes it on the Rust side,
  and the binding's per-call context is restored by guards, so the runtime is as good as before for the ticks
  after.
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
confirm it before starting the next round. A host ends the round's session once its settled state is over
(`round_end`), or, where the peers agree on the last tick, drains it there (`Session::drain`, `is_drained`), and
starts the next round's from that settled state and the shared data.

Done that way (§4.6, "Rounds"): a peer confirms the boundary (its settled state is over) before it starts the
next round, marks its stream, and builds the next round from the settled end state and the setup both peers
agreed (`nettai_match::Set::after`: the settings and score the end hands over; the first round's
navi stats; each player's folder shuffled again by their console's RNG where the round left it, and the console's
frame counter carried on; the battle's RNG drawn from the first round's and the round number, since the
original's init exchange has no counterpart here).
