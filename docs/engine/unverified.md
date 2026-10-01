# What the chip lab hasn't verified

The engine is ported branch by branch from the disassembly; the chip lab's recordings of the original verify what
they reach. This file lists, by area, the branches the engine docs mark unverified, and what has since been
recorded for them. A row leaves the list of open items when a lab scenario reaches it and the engine matches every
frame of it.

## Ruleset

Scenarios named here are in the verification workspace's chiplab library; `flow/` and `custom/` are new folders
for the fight's states and the custom screen. "Matches" means every frame of the recording.

### Covered

| Branch | Scenarios | Result |
|---|---|---|
| The supports (chips.md §2.10): Rush's bite, Beat's theft, Tango's heal and its barrier, each hosted by either side, once a battle | `navicust/rush`, `rush-side1`, `beat`, `beat-side1`, `tango`, `tango-side1` | match |
| The support bug (NaviStats+0x0D = 0xFF): no support comes | `navicust/bug-support` | matches |
| The save's event flag 0x1720 (a NaviCust bug ran: the emotion window flickers, a console RNG1 draw each time) | carried in the trace's setup (`emotion_window_glitches`); `navicust/bug-support`, `bug-hp` | the console's RNG1 keeps step |
| Every NaviCust program without its bug (the earlier recordings had each part on the grid's outer ring, which bugs it) | all of `navicust/` recorded again, plus `poem`, `fldrpak1`, `fldrpak2`, `hp50`..`hp400`, `bugstop` | match |
| The NaviCust bugs by level: panel 1-3, custom 1-3, HP 1-3, buster 1-3, movement, emotion, status, a part on the outer ring | `navicust/bug-*` | match |
| START pause (battle-flow.md §3.5): only the pausing player resumes; both on one tick (side 0 pauses); during a dimming (nothing) | `flow/pause`, `pause-both`, `pause-dimmed` | match |
| A double KO on one tick (§3.6: the round goes by the alive counts, side 0 tested first) | `flow/double-ko` | matches |
| The turn timer from the 15th screen, the damage judge (state 0x18) and its three outcomes; the draw state 0x14 and result code 3 | `flow/judge-win`, `judge-lose`, `judge-draw` | match (10,600 frames each) |
| A lost round (result code 2) | `forms/falzar/ko-lose` | matches |
| Deleted in Beast Out, and at 0 HP in a Cross | `forms/falzar/beast-ko`, `cross-ko` | match |
| Both sides transforming on one turn; a Cross to another Cross (the used one no longer offered); a weakness hit on a Cross Beast; Beast Out out of Full Synchro; the tired turns after Beast Out's three | `forms/falzar/beast-both`, `beast-vs-cross`, `cross-to-cross`, `cross-spout-beast-weakness`, `beast-full-synchro`, `beast-out-spent` | match |
| Invalid chips (custom-screen.md §3.4): a code the chip doesn't have; Mega chips past the Mega level | `custom/invalid-code`, `invalid-mega-count` | match |
| The L message and chip descriptions closed with A and B after a wait | `custom/run-message`, `run-message-b`, `description-keys`, `description-b-held-12` | match |

### Open: recorded, and the engine differs

| Branch | Scenarios | What differs |
|---|---|---|
| The tick a chip description takes keys from: R+6 plus one per line break in its text (R+8 for most chips, R+7 for the Recov chips and the gauge chips), not a constant R+6 | `custom/description-arm-001-6`, `-001-7`, `-09a-6` (and `-5`, `-8`, `-9`, which match) | the engine takes the key two ticks (one) early |
| B held closes a description (11 held ticks once it takes keys) | `custom/description-b-held-30` | not ported |
| The L message's timing (its box opens, its portrait fades in, its text prints three ticks a character, A or held B rushes it) | `custom/run-message-taps-0`, `-1` | the engine's fixed 72 ticks is an estimate |

All three are the chatbox's (`chatbox_onUpdate` and the scripts' commands), which the port stands in for with
constants; the fix is a port of what those two scripts run.

### Not reachable in a netbattle (documented, no scenario)

| Branch | Why |
|---|---|
| NumbrOpn's walk in ChpShufl's re-deal (custom-screen.md §3.7) | Both programs can't be installed: NumbrOpn fills the inner 5x5 of the grid (24 cells compressed), ChpShufl needs 14 cells five wide, and of two overlapping parts only the later one's program applies (probed). |
| The Cross change (battle-flow.md §3.4.1) | The custom screen never sends one: the transform record's +4 has no live writer. |
| The buffered auto-step and its fallback directions (objects-and-player.md §M6.9) | NaviStats+0x11 is never set: no NaviCust bug routine writes it (`byte_813CC18`'s routines write +0x31, +0x24, +0x12/+0x13, +0x63, +0x28, +0x26, +0x14/+0x15, +0x0D, +0x18/+0x16, +0x62 and +0x1A). |
| A step starting in its Land phase (0x10) | Nothing starts a step there. |
| Result codes 4 (escape), 5 (communication error), 9 and 0xA (terminate) | No running in a netbattle (L gives the message); the others are the link's, which the lab's emulated cable never trips. |

### Not attempted yet

| Branch | Note |
|---|---|
| A tag pair straddling the end of the re-deal's walk | Needs the pair's index stale: DustCross's scrap (which doesn't keep it up), then the Cross lost, then a re-deal with the stale index on the folder's last chip. |
| Beat with a Giga chip; Rush with WhiCapsl (the hand left alone) | The dig for a Giga chip takes several turns. |
| Astray steps toward a row or column with no panel to land on | The port panics there (`sub_800D15A` loops); the bug scenarios step where panels are. |
| An uninstall landing on NaviCust programs (bug code 0xF8), and bug code 0xFB | Uninstll folded onto a chip against BodyPack; 0xFB comes from a charged shot kind (NaviStats+0x4F) whose writer isn't found. |
| The Beast Out lock-on's tie-break between several targets | Needs two alive actors on a side. |
