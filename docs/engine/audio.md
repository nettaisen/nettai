# Battle audio (EXE6 US Falzar, BR6E)

The engine plays no sound. Each tick it reports the sound calls the
original makes at that point as typed cues (`nettai_battle::sound`); a
frontend plays them. `nettai-audio` does, with the game's own sound driver
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
| `ScreenVolume { music, screen }` | `sub_802A30C`, `sub_802A362`: volume control on players 31 (`music`) and 22 (`screen`) | a step of the custom screen's dark-chip hover (custom-screen.md §9; no EXE6 chip is dark), heard by the screen's player |

Ids are song-table indices (`SoundId`): music is 0x00..=0x25, effects
0x64 and up. The engine names none: the ruleset plays what content's roles
name (`sounds` and `music` in rules/roles.luau).
Content names its sounds: `asset.sound("exe6:cannon")` is the song the
EXE6 pack's asset index (`assets.toml`) lists under `cannon`, resolved when the
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

The custom screen's own sounds are cues each player's own console makes
(`Battle::sound_for`): the screen records what a tick played
(`ScreenLook::drawn`, `ScreenSound`, in the order the original's states call
`PlaySoundEffect`) and the battle plays each by its role (`sounds.custom_*`,
`program_advance*` in rules/roles.luau): the window sliding in 0x79
(`sub_8026B04`); the cursor 0x7F (a move to another slot, START, the Cross
window's cursor with more than one Cross); SELECT's hide and show 0x80; a
pick 0x81, OK 0x82, a take-back 0x83, what can't be picked or taken back
0x69 (`sub_8028CCC`, `sub_8028D3A`, `sub_8028D6C`, `sub_8029032`); a Beast
Out or Cross taken back 0x1D2; the Cross window opening 0x7A and closing
0x7D, a Cross put on 0x92 (`sub_8027AAE`); L's message 0x7B; R's
description 0x9C and its close 0x9E; Beast Out chosen 0x193 (0x191 on a
Gregar console), 0x81, 0xBC (`sub_802774C`; the BeastOut chip's 0x193 or
0x191, 0xBC: `sub_8027624`); ChpShufl's re-deal 0x182
and its shuffles 0x113 (`sub_802723A`); DustCross's scrap 0x196 a chip, 0x182
when done; the Program Advance animation's chips of the recipe 0x91 and the
Program Advance 0x92 (`sub_802B80C`, `sub_802B920`). The chatbox makes no
sound in battle.

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

## 2. Playing cues: `nettai-audio`

`SoundCalls` turns a cue into the driver calls the game queues for it
(`Request`s: `Start`, `StopAll`, `Tempo`, `Pitch`, `Volume`); it keeps the
current-music byte `PlayMusic` checks. `BattleAudio` runs them:

    battle.tick(&input, events);
    audio.handle(battle.sound_cues());   // queue (32 calls a frame, as sound_8000808)
    audio.tick(&mut samples);            // VBlank, then the queued calls

The game queues sound calls and runs them at the start of the next
frame's main loop, after that frame's VBlank; the driver plays a new song's
first tick on the VBlank after. `tick` keeps that order, so a cue sounds
two frames after its tick, as in the game (the Direct Sound part a frame
later still: the FIFOs play each frame's mix during the next). Output is
32768 Hz stereo (`SAMPLE_RATE`), about 549 samples a frame: the DAC's
65536 Hz averaged down, through a 20 Hz high-pass (the GBA's output
capacitor, which takes out the PSG's offset). `AudioOut` (feature `playback`,
on by default) does the same on the default output device through cpal,
buffering a few frames and resampling to the device rate; the frontend
calls `tick` at the game's 59.73 Hz. `wav::write` saves rendered audio.

## 3. The driver: `m4a`

A port of EXE6's M4A library (the "Sappy" driver, `MKS4AGB`, dated April
2006 in the ROM), routine by routine and in its integer arithmetic, so that
its state and output are the game's (§6). It began as a port of the
arranger's m4a crates (bnmusic, MIT); the sequencer and mixer are now the
game's code.

- **Players.** EXE6 has 32 music players (`bank.players`, the driver's
  player table). Player 31 plays music (8 tracks) and takes any song;
  players 0..=30 play effects (1-3 tracks) and use song priorities: a busy
  player refuses a song of lower priority than its own (`MPlayStart`). Each
  song names its player in the song table (0x6C deletion: player 16,
  priority 255; 0x97 panel crack: player 15, priority 64; the battle music
  0x15: player 31, priority 20). Every frame (`SoundMain`, from the VBlank
  interrupt) the players run in order, 0 first (`MPlayMain` chains them).
- **Sequencer** (`MPlayMain`): the tempo counter (150 a tick), per track
  the gates of its notes, its commands, the LFO; a command that changes a
  track's volume or pitch marks it, and after the ticks `TrkVolPitSet`
  works out the marked tracks' volumes and pitch and passes them to the
  notes they still own (`ChnVolSetAsm`, `MidiKeyToFreq`,
  `MidiKeyToCgbFreq`). A note started on a tick takes its track's values
  and clears the marks, so the track's older notes miss a change made on
  that tick (as in the game). The LFO's falling half takes its phase
  before it wraps, as the game does. Player controls: tempo, pitch and
  volume control, fades (with the game's fade-in and keep-the-tracks
  bits), stop.
- **Channels** (`ply_note`). All players share 4 Direct Sound and 4 PSG
  channels. A note gets a channel with priority = song priority + track
  priority: a PSG note needs its channel free, released, or held by a
  lower note (equal: by a later track); a Direct Sound note takes a free
  channel, else the weakest released one, else the weakest one that
  yields. So effects cut the music's notes; the music gets the channel
  back with its next note.
- **PSG** (`CgbSound`): each PSG channel's envelope counts frames
  (attack, decay, sustain, release, pseudo-echo; every fifteenth frame,
  SoundInfo's c15, it counts twice), and the driver writes the PSG's
  registers: the note's restart with the duty, sweep and length, the
  frequency (rounded for the DAC's resolution for the voices whose type
  has the 0x08 bit), NRx2 with the level and the hardware envelope's
  direction and period, NR51's pan bits, the wave channel's wave RAM and
  level. A channel the hardware has stopped (NR52) ends.
- **Direct Sound** (`SoundMainRAM`): each channel's envelope (0..=255 a
  frame, with the master volume), then its mix into the PCM ring buffer,
  8 bits a sample with wrapping adds, 176 samples a frame at 10512 Hz;
  samples step by `divFreq` times the note's frequency in 23-bit fixed
  point with linear interpolation (reading the byte after the data at the
  end; the "fixed" voices play a sample a sample), loop or end. The ring
  holds nine frames; with reverb, each frame's slot starts as the sum of
  itself and the next slot (nine and eight frames ago) times the level.
- **The hardware** (`m4a::apu`): the PSG as the GB APU of the GBA (duty
  steps, the 512 Hz sequencer's lengths, sweep and envelope steps, the
  noise LFSR, the wave channel's banks), driven by the registers the
  driver writes, applied where its frame starts; the FIFOs play each
  frame's slot during the next frame; the DAC adds them as the GBA does
  (SOUNDCNT_H: PSG at 100%, FIFO A right and B left at full volume; the
  bias clamps to 10 bits). `Driver::take_dac_output` gives the DAC's
  samples at its rate (65536 Hz for EXE6's 8-bit setting), in mGBA's scale;
  `take_output` averages them to 32768 Hz through the output capacitor (a
  20 Hz high-pass that takes out the PSG's offset).
- **MEMACC** (0xB9): all 18 operations on the driver's memory area
  (`Driver::memory`, `set_memory`): set, add, subtract (by a value or by
  another byte of the area) and jump if equal, not equal, greater, at
  least, at most, less (than a value or a byte). EXE6's game area is the
  16 bytes at 0x02010B90; two map songs (0x11, 0x23) mark their phrases
  in byte 0 so that the game's `sub_8000822` switches music on a phrase
  boundary. The battle uses neither.

What the port doesn't play, none of which any EXE6 song uses (the
extractor reads all 399 songs): the commands PORT (0xCC, a write to a
sound register) and XCMD's tone changes (0x01 wave, 0x02 type, 0x04-0x07
the envelope, 0x0A length, 0x0B sweep); XCMD 0x0C and up jump into data in
EXE6's driver. A ROM with one of them has the song left out at extraction,
with the command named.

## 4. Sound data

`exe6-extract content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> <dir>` reads the Falzar ROM's M4A data
(`m4a::rom::extract`) into a typed `m4a::SoundBank` (songs as command
lists with running status resolved and jumps as command indices,
voicegroups, drum kits, key splits, samples and PSG waves, the mixer
settings and the player table) and writes it into the content pack as
open, editable files: songs as MIDI in mid2agb's conventions, voicegroups
as TOML, samples as WAV (`docs/design/asset-formats.md`). nettai-content reads
them straight back into a `SoundBank` (`nettai_content::pack::load_sound`),
and every song renders the same samples from them as from the ROM.
`SoundBank::validate` checks every reference in a bank before the driver
trusts it. The pack holds the game's recordings, so it is never committed
(`data/content/` is gitignored), and nothing reads the ROM at run time.

Every EXE6 song reads (399). The sound files' version 2 carries what the
exact driver needs beyond version 1: a PSG voice's sweep, its fixed
frequency bit and length, the byte after a sample's data (one EXE6 sample
has another than the usual), and the DAC's resolution; a version 1 pack
loads with a warning and plays without them. MEMACC's set, add and
subtract are mid2agb's controllers (CC 13 the operation, CC 14 the
address, CC 12 the operand, which runs it); a conditional MEMACC jumps on
what the game wrote, so a song with one can't be a timeline and is left
out of a pack (EXE6 has none). A track whose running status would depend on
the path into a jump target is refused at extraction (none in EXE6).

## 5. Hearing it

    cargo run -p exe6-extract -- content <falzar-us> <gregar-us> <falzar-jp> <gregar-jp> data/content/exe6
    cargo run -p nettai-audio --example trace_audio -- <trace.jsonl> data/content/exe6
    cargo run -p nettai-audio --example trace_audio -- <trace.jsonl> data/content/exe6 --wav out.wav --frames 600
    cargo run -p nettai-audio --example play_song -- data/content/exe6 0x15,0x94 --every 120

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
function), over every frame the engine reproduces, the custom screen's own
sounds included.

- The two golden traces, over every frame of every round: 89 calls over
  2405 frames and 1360 calls over 57,331 frames, call for call.
- Every chip-lab scenario, each recorded with the sound calls the
  original queued: 5163 scenarios (every chip, Program Advance, form,
  link navi, NaviCust program, stage and ruleset scenario the lab has),
  96,393 calls over 4,169,142 frames, every one call for call (Beast Over's
  rumble and the burners' roar included, with the attack's count above).
  This comparison is a standing gate of the chip lab: a new recording
  carries its calls, and a sound that differs fails it.
- Every sound the content names (148 names) is in the pack's index,
  has a song, starts on the driver and makes sound; so does every number
  the engine's own routines play.
- Under rollback (the golden traces replayed through rollback peers at
  several latencies): the cues of a peer's confirmed frames are the plain
  replay's, frame for frame, and what it played and didn't take back is the
  same cues (`cues::CueTracker`, docs/design/rollback.md).
- The sound itself, against recordings of the original under emulation
  (each song started on its own on a freshly booted game; the driver's
  state after every SoundMain and mGBA's audio output): the battle music
  0x15, the winner's 0x1F and loser's 0x1A music, the MEMACC map song
  0x11, and ten effects that between them play every voice type (Direct
  Sound 0x6B, 0x71, 0x8F; square 1 with a sweep 0x6C, without 0x84, 0x86,
  0x9F; square 2 0x6C, 0x9D; wave 0x9D, 0xDB; noise 0x97; the effects'
  PSG voices all with the fixed-frequency bit). 15,070 frames, with 21,831
  channel-frames of Direct Sound and 37,821 of PSG sounding.
  - The driver's state matches on every frame: each Direct Sound
    channel's status, keys, volumes, envelope, frequency and place in its
    sample; the 176 bytes a side it mixed into the PCM buffer; each PSG
    channel's status, envelope level, goal and counter, frequency, pan and
    NRx4; and the PSG registers the driver wrote.
  - The output: the Direct Sound effects come out of the DAC sample for
    sample to 99.8% (once lined up: the FIFOs' latency and the timer's
    phase are the game's); what differs is the emulator's timer events
    landing a sample either side of the DAC's grid now and then. The PSG
    can't match sample for sample: where a square is in its duty cycle goes
    back to the game's boot, and the driver's register writes fall where
    the game's CPU gets to them in VBlank. Compared as loudness a frame at a
    time, the PSG effects agree to 17-34 dB and the music to 16-25 dB
    (the Direct Sound effects to 49-56 dB).
- What a frontend plays (the golden traces' rounds through `SoundCalls` and
  the driver, as `BattleAudio` does): the round's battle music (the link
  battle's `music.link_battle`) starts with the intro, sounds and plays on
  to the result, then the winner's or the loser's music
  (`music.winner`, `music.loser`) takes over; the low-HP alarm (92 times
  over soundmod's rounds) and the pause sound (the traces never pause: the
  test pauses and resumes every 25 seconds) start every time and the music
  goes on under them, never restarted.
- Under rollback, the sound a peer's player hears: each peer's cue
  actions, played on the frame it decided them, through `SoundCalls` and
  the driver: the same music changes as the plain replay, each at most
  the latency later, and every song the replay starts started once within
  that (the rest predictions stopped again). On the golden traces at
  latencies of 2, 5 and 10 frames (with 1-3 of jitter): every round's
  three music changes in step, effects at most 7 frames late at 10 frames'
  latency, and up to 17 effects in a round started on a prediction and
  stopped again.

In this repo, tests cover the cue plumbing on a battle built in code, the
driver on synthesized songs (priorities, channel stealing, controls, the
sequencer) and the ROM reader on a synthesized ROM image. The frontend's
`--audit` plays a trace's every cue into nothing and reports a cue whose
song the pack doesn't have.
