# Battle audio (BN6 US Falzar, BR6E)

The engine plays no sound. Each tick it reports the sound calls the
original makes at that point as typed cues (`bn6_battle::sound`); a
frontend plays them. `bn6-audio` does, with the game's own sound driver
(the `m4a` crate) and sound data (the sound of a content pack extracted
from the user's ROM). Cues are output only: nothing in the simulation reads them.

## 1. Cues

`Battle::play_sound(cue)` records a cue; `Battle::sound_cues()` returns the
last tick's, in the order the game makes the calls (the list is cleared at
the start of every tick).

| Cue | The game's call | Where |
|---|---|---|
| `Effect(id)` | `PlaySoundEffect(id)` = `m4aSongNumStart(id)` | the engine's routines below, and the content's (`play_sound`) |
| `Music(id)` | `PlayMusic(id)`: nothing if `id` is GameState's current-music byte; else sets it, and `m4aMPlayAllStop` for 0x63, `m4aSongNumStart(id)` otherwise | intro init `sub_80091F0` (0x15 in link battles, else the settings' music unless 0x63); win `sub_80081A4` (0x1F, or 0x19 with effects bit 1); loss `sub_800825A` (0x1A, link only) |
| `StopMusic` | `musicGameState_8000784`: `m4aMPlayAllStop`, current music = 0xFF | fade-out done `sub_80094DA` |
| `Pinch(true/false)` | `sub_8009158`: pitch control (all tracks, +0x100 or 0) and tempo control (0x11A or 0x100) on music player 31 | after the mode handler, link battles, when the local navi's HP crosses MaxHP/4 |
| `RestoreVolume` | `sub_802A3CC`: volume control 0x100 on players 31 and 22 | custom screen closes (`sub_8026A6C`) |

Ids are song-table indices (`SoundId`): music is 0x00..=0x25, effects
0x64 and up. The engine names none: the ruleset plays what content's roles
name (`sounds` and `music` in rules/roles.luau).
Content names its sounds: `asset.sound("cannon")` is the song the
pack's asset index (`assets.toml`) lists under that name, resolved when the
content loads (a name the pack doesn't list is a load error), so a content
sound is a song of whatever pack is loaded.

Effects the engine emits, by routine: navi fade-in 0x94 (`sub_80163B4`);
hit 0x6B on the local player's navi, else 0x6D (`applyDamageToPlayer_801ba12`);
custom-HP bug 0x6B (`sub_8013FD0`); counter 0x86 (`sub_801A45C`); flash
end while invisible 0x94 (`sub_801A5EE`); flag-4 timer end 0x94
(`sub_8010162`); deletion 0x6C (`sub_801746E`); freeze 0x118, bubble 0x12D,
bubble pop 0x124 (`sub_8017688`, `sub_8017768`); cursor trap 0x8E
(`sub_802CFF8`); heat trap 0x6E (`sub_802CEF4`); drain heal 0x8A
(`sub_801A324`); guard spark 0x6E (`object_spawnHiteffect`); can't jack in
0x69 (`sub_8012FC8`); buster charge 0x71/0x72 (`sub_80E0F5E`); custom gauge
full 0x8F (`sub_801C470`); panel crack/break 0x97 (`object_crackPanel`,
`sub_800C380`); obstacle hit 0x85 (`sub_801B394`); rock landing 0xC0
(`sub_80CFAC0`); rock break, the kind's sound (`sub_80CFB2C`); pause and
resume 0x9F (`sub_801E15C`, `sub_80083E4`); the HP box's alarm 0x84, every
45 ticks a console's own navi is at a quarter of its HP or less
(`sub_801C840`, heard on that console only).

The HUD's warning marker (`sub_800AE90`: the gauge chips', VDoll's marks,
LifeSync's) sounds on every 16th frame of the console's own frame counter,
which counts frames since the match began whatever the battle does
(`Console::frames`, from `ConsoleSetup::frames`): content calls
`battle.warn(sound, at, side)` every tick the marker shows, and the engine
makes the cue on those frames, for each console that shows it.

Not emitted: the custom screen's own UI sounds (cursor 0x7F, select
0x81/0x82, open 0x79, a Cross chosen 0x92, Beast Out, ...). The engine runs
the screen (docs/engine/custom-screen.md), but its sounds belong to the
console showing it, and nothing draws the screen yet either.

Two sounds depend on a word the original's actions share. Every action
keeps a count in its attack variables (AIAttackVars+0x12: shots, swings,
slashes, a hold's ticks, the recovery after a shot) and nothing clears it
between actions, so a routine that reads it before writing it reads what
an earlier action left. The engine keeps that word as the navi's
`attack_count` (`AttackVars::count`), which every action that has a count
keeps it in (44 routines, all content's), and so:

- Beast Over's rumble (0x19A) sounds on the vanish's ticks 0x35 and 0x25
  only when the count is 0: the game compares the timer as a 32-bit word,
  whose upper half is the count (`sub_80151D4`; a buster shot leaves 2,
  and then it sounds once, at the vanish's start);
- a burner's roar (0x12B: FireBrn's flames, HeatCross's charged shot)
  sounds every 16 ticks of the count, which the burn counts on from what
  the last action left (`sub_80ECD44`).

The low-HP latch (BattleState+0x20) isn't reset between rounds in the
game: rounds after the first start with it set, so they emit only
`Pinch(false)` on their second tick. A frontend starting a round should
carry it over like the other round counters.

## 2. Playing cues: `bn6-audio`

`SoundCalls` turns a cue into the driver calls the game queues for it
(`Request`s: `Start`, `StopAll`, `Tempo`, `Pitch`, `Volume`); it keeps the
current-music byte `PlayMusic` checks. `BattleAudio` runs them:

    battle.tick(&input, events);
    audio.handle(battle.sound_cues());   // queue (32 calls a frame, as sound_8000808)
    audio.tick(&mut samples);            // VBlank, then the queued calls

The game queues sound calls and runs them at the start of the next
frame's main loop, after that frame's VBlank; the driver plays a new song's
first tick on the VBlank after. `tick` keeps that order, so a cue sounds
two frames after its tick, as in the game. Output is 32768 Hz stereo
(`SAMPLE_RATE`), about 549 samples a frame. `AudioOut` (feature `playback`,
on by default) does the same on the default output device through cpal,
buffering a few frames and resampling to the device rate; the frontend
calls `tick` at the game's 59.73 Hz. `wav::write` saves rendered audio.

## 3. The driver: `m4a`

A port of the arranger's m4a crates (bnmusic, MIT). The mixer (Direct
Sound envelopes and resampling into the 8-bit mix buffer with its reverb,
PSG envelopes and synthesis, output levels) is theirs. The sequencer is
rewritten after the GBA driver so that many songs play at once:

- **Players.** BN6 has 32 music players (`bank.players`, the driver's
  player table). Player 31 plays music (8 tracks) and takes any song;
  players 0..=30 play effects (1-3 tracks) and use song priorities: a busy
  player refuses a song of lower priority than its own (`MPlayStart`). Each
  song names its player in the song table (0x6C deletion: player 16,
  priority 255; 0x97 panel crack: player 15, priority 64; the battle music
  0x15: player 31, priority 20).
- **Channels.** All players share 4 Direct Sound and 4 PSG channels. A note
  gets a channel as `ply_note` decides, with priority = song priority +
  track priority: a PSG note needs its channel free, released, or held by
  a lower note (equal: by a later track); a Direct Sound note takes a free
  channel, else the weakest released one, else the weakest one that
  yields. So effects cut the music's notes; the music gets the channel
  back with its next note.
- **Sequencer.** `MPlayMain` per player per frame: the tempo counter,
  per-track gate countdown, commands, LFO, then volume and pitch
  (`TrkVolPitSet`) for the notes each track still owns. Player controls:
  tempo, pitch and volume control, fade out, stop.

Checked against the arranger's offline renderer on the battle music
(0x15): the renders agree closely, and the differences found come from
that renderer applying control changes (pan, bend, ...) one tick early,
where the GBA driver applies them on their tick.

## 4. Sound data

`bn6-extract content <rom> <dir>` reads the ROM's M4A data
(`m4a::rom::extract`) into a typed `m4a::SoundBank` (songs as command
lists with running status resolved and jumps as command indices,
voicegroups, drum kits, key splits, samples and PSG waves, the mixer
settings and the player table) and writes it into the content pack as
open, editable files: songs as MIDI in mid2agb's conventions, voicegroups
as TOML, samples as WAV (`docs/design/asset-formats.md`). bn6-content reads
them straight back into a `SoundBank` (`bn6_content::pack::load_sound`),
and every song renders the same samples from them as from the ROM.
`SoundBank::validate` checks every reference in a bank before the driver
trusts it. The pack holds the game's recordings, so it is never committed
(`data/content/` is gitignored), and nothing reads the ROM at run time.

Every BN6 song reads (397) except two map songs that use MEMACC (0x11,
0x23), which the port doesn't play. A track whose running status would depend on
the path into a jump target is refused at extraction (none in BN6).

## 5. Hearing it

    cargo run -p bn6-extract -- content <rom> data/content/bn6
    cargo run -p bn6-audio --example trace_audio -- <trace.jsonl> data/content/bn6
    cargo run -p bn6-audio --example trace_audio -- <trace.jsonl> data/content/bn6 --wav out.wav --frames 600
    cargo run -p bn6-audio --example play_song -- data/content/bn6 0x15,0x94 --every 120

`trace_audio` replays a golden trace's rounds with their recorded inputs,
on the pack's battle data, prints each cue with its frame and plays them in real time (or renders a
WAV). A round stops where the engine leaves the recording (with
`--keep-going`, where it panics on something not implemented yet).

## 6. Verified

The cue ids and frames are checked against the original by the
verification suite outside this repo. It replays a trace, turns each tick's
cues into driver calls with `SoundCalls`, and compares them with the calls
the original queued on the same frames, recorded from the original under
emulation (the queue at 0x0200A490: count, then 16-byte entries r0, r1, r2,
function), over every frame the engine reproduces. The custom screen's own
sounds are left out.

- The two golden traces, over every frame of every round: 89 calls over
  2405 frames and 1360 calls over 57,331 frames, call for call.
- Chip-lab scenarios recorded with their sound calls: 4553 of the lab's
  scenarios (84,832 calls over 3,667,325 frames), every one call for
  call.
- Every sound the content names (148 names) is in the pack's index,
  has a song, starts on the driver and makes sound; so does every number
  the engine's own routines play.
- Under rollback (the golden traces replayed through rollback peers at
  several latencies): the cues of a peer's confirmed frames are the plain
  replay's, frame for frame, and what it played and didn't take back is the
  same cues (`cues::CueTracker`, docs/design/rollback.md).

In this repo, tests cover the cue plumbing on a battle built in code, the
driver on synthesized songs (priorities, channel stealing, controls, the
sequencer) and the ROM reader on a synthesized ROM image. The frontend's
`--audit` plays a trace's every cue into nothing and reports a cue whose
song the pack doesn't have.
