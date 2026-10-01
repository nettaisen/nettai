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
| The supports (chips.md §2.10): Rush's bite, Beat's theft of a Mega and of a Giga chip, Tango's heal and its barrier, each hosted by either side, once a battle | `navicust/rush`, `rush-side1`, `beat`, `beat-side1`, `beat-giga`, `tango`, `tango-side1` | match |
| The support bug (NaviStats+0x0D = 0xFF): no support comes | `navicust/bug-support` | matches |
| The save's event flag 0x1720 (a NaviCust bug ran: the emotion window flickers, a console RNG1 draw each time) | carried in the trace's setup (`emotion_window_glitches`); `navicust/bug-support`, `bug-hp` | the console's RNG1 keeps step |
| Every NaviCust program without its bug (the earlier recordings had each part on the grid's outer ring, which bugs it) | all of `navicust/` recorded again, plus `poem`, `fldrpak1`, `fldrpak2`, `hp50`..`hp400`, `bugstop` | match |
| The NaviCust bugs by level: panel 1-3, custom 1-3, HP 1-3, buster 1-3, movement, emotion, status, a part on the outer ring | `navicust/bug-*` | match |
| Steps gone astray (the movement bug) toward every edge, with and without a panel to go to | `navicust/bug-movement-edges` | matches |
| FlotShoe on every stage (grass, ice, poison, volcano, holy, cracked, holes, roads) and AirShoes over holes, cracked, ice and poison panels | `navicust/flotshoe-stage-*`, `airshoes-stage-*` | match |
| The four bugs a BugBomb gives (bug codes 0x18, 0x19, 0xF5 and stat 0x14), one after another on one navi, and the navi living with them (blank shots, the HP drain, the panel trail, the custom screen's drain) | `chips/0x043-bugbomb/four` | matches |
| An uninstall (bug code 0xF8) landing on BodyPack's programs (UnderShirt stays), on a navi in Beast Out (the form's shoes come back), on a link navi (it keeps what it has), and on UnderShirt at 30 HP | `chips/0x0b9-uninstll/folded`, `folded-beast`, `folded-navi`, `folded-undershirt` | match |
| START pause (battle-flow.md §3.5): only the pausing player resumes; both on one tick (side 0 pauses); during a dimming and once the battle is over (nothing) | `flow/pause`, `pause-both`, `pause-dimmed`, `pause-over` | match |
| A screen confirmed with nothing picked keeps the hand's chips; only Beast Out picked empties it | `flow/keep-hand` | matches |
| The custom screen opened while an attack is under way (a Vulcan mid-burst), and by both players on the tick the gauge fills (L and R) | `flow/custom-open-busy`, `custom-open-both` | match |
| Anger from a 300-damage hit (the next chip doubled, the anger spent) and its 600 ticks running out; anger from a counter's paralysis | `flow/anger-big-hit`, `anger-runs-out`, `synchro-lost` | match |
| Full Synchro from a counter hit: the next chip doubled and the synchro spent; lost to a plain hit | `flow/synchro-used`, `synchro-lost` | match |
| A navi deleted by a navi chip's hit during its dimming | `flow/ko-dimmed` | matches |
| A double KO on one tick (§3.6: the round goes by the alive counts, side 0 tested first) | `flow/double-ko` | matches |
| The turn timer from the 15th screen, the damage judge (state 0x18) and its three outcomes; the draw state 0x14 and result code 3 | `flow/judge-win`, `judge-lose`, `judge-draw` | match (10,600 frames each) |
| A lost round (result code 2) | `forms/falzar/ko-lose` | matches |
| Deleted in Beast Out, and at 0 HP in a Cross | `forms/falzar/beast-ko`, `cross-ko` | match |
| Every Cross knocked out by its weakness (the next screen no longer offers it) | `forms/falzar/cross-{spout,tomahawk,tengu,ground,dust}-weakness`, `forms/gregar/cross-{heat,elec,slash,erase,charge}-weakness` | match |
| A Cross Beast's three turns running out | `forms/falzar/cross-spout-beast-spent` | matches |
| Both sides transforming on one turn; a Cross to another Cross (the used one no longer offered); a weakness hit on a Cross Beast; Beast Out out of Full Synchro; the tired turns after Beast Out's three | `forms/falzar/beast-both`, `beast-vs-cross`, `cross-to-cross`, `cross-spout-beast-weakness`, `beast-full-synchro`, `beast-out-spent` | match |
| Invalid chips (custom-screen.md §3.4): a code the chip doesn't have; Mega chips past the Mega level | `custom/invalid-code`, `invalid-mega-count` | match |
| The chatbox the custom screen waits on (custom-screen.md §3.5): the tick a description takes keys from, by its text's lines (Cannon R+8, Recov10 R+7, the invalid chip R+6, a Cross R+8); B held; the L message's printing, rushed by A or held B, for MegaMan and three link navis | `custom/description-arm-*`, `description-invalid-*`, `description-cross-*`, `description-b-held-12`, `-30`, `description-keys`, `run-message`, `-b`, `-wait`, `-taps-0`, `-taps-1`, `-b-held`, `run-message-navi-*` | match, since the chatbox's port (the engine took a description's key from R+6 whatever its lines, had no held B, and estimated the message at 72 ticks) |

### Not reachable in a netbattle (documented, no scenario)

| Branch | Why |
|---|---|
| A tag pair straddling the end of the re-deal's walk (custom-screen.md §3.7) | The walk overruns when it meets the pair's index with one entry left to count. OK takes one off the index for every chip it takes out of the folder, and the scrap changes neither, so the chips left less the index stay what the shuffle made them: 30 less an index of 1 to 20, at least 10. |
| NumbrOpn's walk in ChpShufl's re-deal (custom-screen.md §3.7) | Both programs can't be installed: NumbrOpn fills the inner 5x5 of the grid (24 cells compressed), ChpShufl needs 14 cells five wide, and of two overlapping parts only the later one's program applies (probed). |
| The Cross change (battle-flow.md §3.4.1) | The custom screen never sends one: the transform record's +4 has no live writer. |
| The buffered auto-step and its fallback directions (objects-and-player.md §M6.9) | NaviStats+0x11 is never set: no NaviCust bug routine writes it (`byte_813CC18`'s routines write +0x31, +0x24, +0x12/+0x13, +0x63, +0x28, +0x26, +0x14/+0x15, +0x0D, +0x18/+0x16, +0x62 and +0x1A). |
| A step starting in its Land phase (0x10) | Nothing starts a step there. |
| Result codes 4 (escape), 5 (communication error), 9 and 0xA (terminate) | No running in a netbattle (L gives the message); the others are the link's, which the lab's emulated cable never trips. |
| Rush with chip 0x17E (the hand left alone) | Chip 0x17E, the other WhiCapsl, comes in no code: it can't be in a folder. |
| A navi appearing mid-battle (`sub_80164A0`) | Only for actors whose AIData+2 is set, which a netbattle's players' isn't. |
| The Beast Out lock-on's tie-break between several targets | Its candidates are a side's alive-actor slots, and a netbattle fills one a side. |

### Not attempted yet

| Branch | Note |
|---|---|
| Bug code 0xFB (the body programs go, UnderShirt too) | It comes from a charged shot kind (NaviStats+0x4F), whose writer isn't found among the NaviCust's routines. |
| The descriptions of DblBeast, Gregar and Falzar | Their scripts print a value with a command (`FF`) the chatbox's port counts as text; Giga chips, so a dig of several turns. |
