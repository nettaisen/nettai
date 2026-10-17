# Side symmetry: what the original did by side, and what nettai does

A battle and its mirror image should play as mirror images of each other: the players change sides (with
everything each brings: folder, navi, stats, console), the field is mirrored (panel X to 7 − X, the owners and
the panel types with it: a road to the left is one to the right), each player presses the same buttons (the engine
reads Left and Right as back and forward on either side), and the stage's actor list keeps its order relative to
the navis (the navi that spawns first still spawns first: the netbattle stages come in such pairs). Then every
tick of the mirror should be the mirror of the round's: the same HP, statuses, timers, hits, panels and RNG draws,
the positions and velocities mirrored.

The original is not quite so. Where its code reads an X as it is (a tie broken toward the right, a line between
two columns put in the right-hand one, a rounding toward minus infinity, a list in the field's order taken from
its first entry, a screen edge at a fixed X), side 1 gets something other than the mirror of side 0's. nettai makes
those side-fair: **one side's behavior, the canonical side's, is every side's**: an object of the canonical side
does as the original does, and one of the other side does the mirror image of that. By the user's choice
(2026-10-17, "side 0 everywhere") the canonical side is side 0.

Left alone on purpose (the user's choice, 2026-10-17): the update order. Objects update in one list, and the
navi the stage's actor list spawns first acts and resolves its hits first each tick; spawns go right after their
spawner. The stages come in pairs that differ only in that order, which the mirror keeps. No per-round priority
is picked.

## The canonical side

`nettai_content_api::CANONICAL_SIDE` (crates/nettai-content-api/src/types.rs) is the canonical side, 0 or 1; it is
0. It is an internal constant of the engine, for switching the rule on a rebuild: no setup, rule, match key, replay,
wire format, digest or screen has it. Everything side-fair reads it through one function, `side_sign(side)`: +1
for the canonical side, −1 for the other, the sign that turns an X into the canonical side's frame. The Rust
rules call it directly (`PanelPos::under`, `passed_x`, `MoveDirection::BySide`); content reads it through the core
API only, `field.side_sign(side)` and `field.canonical_side()` (content/nettai/core.d.luau), never by a number of
its own.

To build nettai with side 1's behavior for every side, set `CANONICAL_SIDE` to 1 and rebuild. The engine's tests
are written relative to the constant and pass either way (checked by hand on 2026-10-17 with it set to 1: all of
nettai-battle's, nettai-content-api's and nettai-luau's, and the WindRack and barrier recordings, which then push
on both sides). bn6battle-verify's known deviations (`src/deviations.rs`) are the default build's: with side 1
canonical, the recordings whose side 0 ran a changed branch would differ instead.

## The mirror check

bn6battle-verify's trace-tests (`src/mirror.rs`, `tests/mirror.rs`) plays each recorded round (the chip labs of
the three games, EXE6's golden traces and 2022 rounds) twice: as recorded, and as its mirror image (sides swapped,
the stage the mirror one, the same buttons). After every tick it compares the two runs: the RNGs, the round and
fight state, every object (slot by slot in the update order; as sets of objects once only their order differs,
for objects of no side spawned along the field's columns), the panels, each side's state. It reports the first
difference of each round, grouped by where it is.

    MIRROR_SETS=exe6-lab,exe5-lab,exe4-lab,exe6-golden,exe6-legacy \
        tools/traces-against.sh <engine checkout> --release --test mirror -- --ignored

What it leaves out is presentation: what is drawn (the `VISIBLE` flag, sprites, the HUD, sounds, the cameras'
shake offsets), where a drawn thing or an object without a collision stands (a hit spark's random jitter is in
screen pixels; what such an object does in play shows in what it spawns), a body's pixel of jitter while it
shakes in a dimming, a body parked off the field at the same X whatever its side, the judge's rolling digits, and
values the code never reads (a bug code's garbage argument, header bits 0x40 and 0x80). Two values are compared
loosely: EXE4's direction after a move is absolute and after a slide by the side, and which wrote it isn't kept, so
its 3 and 4 compare as one; and the attack's marker word, each action's own packing, is left out (what it decides
shows in what the action places and hits). A body shaking in place by whole screen pixels (a heavy hit's shake,
Count's rain) may stand up to 4 pixels off the mirror's place. Its list of allowed differences (`ALLOWED`, `SCOPED`)
is §4 and §5.

The scenarios only find what they exercise; the code was audited besides for the shapes in §3.

## 1. What nettai changes

Each entry: the routine (the original's name or address), what each side did in the original, what nettai does,
where. "As the canonical side's" means: the original's for an object of the canonical side, its mirror image for
one of the other side.

### 1.1 The panel under a position (all games)

`sub_800E258` divides `(X >> 16) + 160` by 40: a position on the line between two columns is in the right-hand
one, ahead of side 0 and behind side 1, so a side-1 object crossing a line entered the next panel a tick after a
side-0 one (a fast thunder ball, 1 pixel a tick from a panel's center, every 40 ticks). Off the field the two
sides' columns ran on differently too (side 0 stays on 0 for 80 pixels, side 1 goes 7 then 8). nettai's
`PanelPos::under(x, y, side)` (nettai-content-api) divides the canonical side's X as it is and the other side's
mirrored; `set_panel_from_coordinates` and the navis' slides and drags use it, and the content's own copies of the
routine call `field.panel_at(x, y, side)` (the link chips, ChargeMan and his car, the yoyos, AirSpin's top and
whirl, AirHocky's puck, JudgeMan's books, MrkCan, EXE4's DarkNeo and gear). With side 0 canonical, a line between
two columns is in the column toward the other side, for both sides.

**WindRack against a barrier up close (EXE5, EXE6).** The case the user raised. WindRack hits the column ahead
and blows a gust down each row from its center, 10 pixels a tick. Recorded in the original (bn6battle-verify's
chip lab, `chips/0x050-windrack/barrier`, `barrier-side1`, `barrier-far`, `barrier-far-side1`): a navi with
Barr100 right in front of a side-0 user is not pushed; in front of a side-1 user it is; from further away both are.
The cause is the line: a side-0 gust reaches the line 20 pixels on, its second tick, which the original puts in
the next column, so it is on the barrier's column for one tick; a side-1 gust at the mirrored X stays in the
column, two ticks. The first tick's hit pops the barrier (wind), which absorbs what reaches it that tick and the
next; the second gust tick of a side-1 user's lands after it and pushes. With side 0 canonical, up close neither
side's navi is pushed; from further away both are (the gusts meet the navi on their first tick there, the
barrier long gone). The engine's test `wind_racks_gusts_meet_the_navi_ahead_as_long_from_either_side` holds the
cause, the gusts' ticks on the column ahead by the user's side.

### 1.2 A move passing a point (all games)

`sub_800E6E8`: a move from `old` to `new` passed `target` when it lies in (old, new] moving right, in (new, old]
moving left: moving right, reaching the point counts and starting on it doesn't; moving left the other way round.
So a side-1 flying shot (TrnArrw, the arrows) spent its range a panel's center sooner than side 0's, a side-1
SlashCross wave its reach, and a side-1 navi dragged back arrived a tick sooner. nettai's `passed_x(new, old,
target, side)` takes the X in the canonical side's frame; along Y the original's `passed`. The content's copies
call `field.passed(new, old, target, side)` (the flying shots of the three games, CrakShot, AirSpin's top, EXE5's
AquaWhirl, SlashCross's wave, EXE4's WindMan's tornado and gear); the engine's drags and obstacle slides use it.
With side 0 canonical, a move ahead counts on reaching and one back on leaving, for both sides.

### 1.3 A move's direction (EXE5, EXE6)

`sub_800E994` (the rules' `move_direction = "by_side"`), what a collision record keeps as its last move and ice
reads: up, down, back or forward by the side, other for a move of two panels or more right or down. A move of two
panels to the right was other for side 0 (ahead) and back for side 1; one to the left back for side 0 and forward
for side 1. nettai tests the X in the canonical side's frame (`MoveDirection::of`): with side 0 canonical, two
panels ahead is other and two back is back, for both. (EXE4's `absolute` direction, 3 left and 4 right whatever
the side, is a mirror image as it is: the mirror's left is the round's right.)

### 1.4 The thunder ball's re-aim (EXE4, EXE5, EXE6)

`sub_80C956C` (content/exelib/thunder/ball.luau, `aim`): every panel the ball aims anew at the nearest column with
an enemy body; along its row toward it, or along the column when the target is in it. With the target on the
ball's own panel, it goes right (`tx >= x`): ahead for a side-0 ball, back for a side-1 ball. Every ball goes as
the canonical side's: with side 0 canonical, toward the other side. BgDthThd's charged shot, Thunder, DarkThnd,
ElecMan's and the other balls of the shared kind. (The user first chose side 1's for BgDthThd, then "side 0
everywhere".)

### 1.5 Panel lists taken in order (EXE4, EXE5, EXE6)

`object_getPanelsFiltered` and `object_getPanelsExceptCurrentFiltered` list the panels from (6, 3) back to (1, 1),
from side 1's back column. A list gone through in order, its first entry taken, or shuffled (`sub_8000C72`: as many
swaps of two random entries as entries, which leaves the first entries likelier where they were) so gave side 1
something other than side 0's mirror. `panels.filtered` and `panels.filtered_except` (content/exelib/panels.luau)
take an optional side: the canonical side's list in the original's order, the other side's with its columns
reversed. Passed where the order counts: DblBeast's targets, ElementMan's, ElecMan's charged shot, ChargeMan's
chip, the Gregar and Falzar chips' aim (lib/beast_chips/targets), Geddon's quake (and the capsules'), the land
mines, EXE5's Meddy, EXE4's WoodMan and BigHamr. A single uniform pick from a list is left in the field's order
(§4).

### 1.6 Aims (EXE4, EXE5, EXE6)

`calcAngle_800117C`, `sub_80011A0`, `sub_8001330` and `sub_800120E` (content/exelib/trajectory.luau) round toward
minus infinity (arithmetic shifts of the distances, the products and the angle), so an aim to the left came out a
sixteenth of a pixel a tick or so off the mirror of the same aim to the right, and crossed a column's line on
another tick. Each helper takes an optional side; the other side's than the canonical works on the mirror image
(X negated going in, the X or the angle mirrored coming out). Every aim passes its object's side: the snakes,
LilBoiler, BlkBomb, FlashBomb, RollngLog, Django, ChargeMan's rocks, HeatMan's jump, ColorPoint, VDoll, the
supports' heal, EXE5's Quake, CactBall, CactDance, Meddy's capsules, NapalmMan's bombs, CrakBomb, EXE4's Binder
and SeekBomb.

### 1.7 A stage's boulder's flags (EXE4, EXE5, EXE6)

`sub_80D2430` reads the new boulder's header flags through its panel's column: a byte of the BIOS's last fetched
word, by the column's low two bits. On column 3 (side 0's front) the boulder is in use and shown, on column 4
(side 1's front) not in use (its sprite never steps); on column 2 bit 0x80 is set and on 5 not. The canonical
side's boulder reads by its column, the other side's by its mirrored column (content/exelib/boulder). EXE5's and
EXE4's stages put boulders on columns 3 and 4; EXE6's on 2 and 5, where only bit 0x80 changes, which nothing reads
(and the recordings' comparison skips).

### 1.8 BlzrdBal's roller (EXE6)

The roller sits 20 pixels ahead of the snowball: the ball's whole pixels plus 20, its own fraction kept, which its
spawn took from the ball's sprite flip. Neither is the mirror of side 0's on side 1, so a side-1 roller crossed a
column's line on another tick. The other side's spawn and follow are the canonical side's in the mirror image
(content/exelib/blzrdbal/roller.luau).

### 1.9 The hook's flames (EXE5, EXE6)

`sub_80DE818` starts a flame from its arc table's row by its facing. The row for a flame facing left isn't the
mirror image of the other everywhere (BigHook's second swing starts at 0x8A, where the mirror of 0xD2 is 0xAE), and
its angle (the timer's high byte), turning the other way, rounds the other way. nettai swings a flame facing the
other way than the canonical side's as the mirror image of one facing its way: that row's angle mirrored, its low
byte full so that it rounds as that row's does (content/exelib/flmhook/fire.luau). With side 0 canonical, the
table's second row is read by nothing.

### 1.10 The screen's edges (EXE4, EXE6)

The original's screen runs from X −160 to 159 whatever the side, so a thing that ends past an edge (or past a
column at a fixed X) went a pixel or a tick further toward one side than the other. nettai reads the X in the
canonical side's frame: EXE4's TwinFang fangs, PropBom bombs, PanlSht panels and Binder, and the HolyDrem shot
(content/exelib/holydrem). EXE4's TopMan's top, which goes in a puff over a panel that isn't solid but on column 7
and leaves past the right edge, takes the mirrored column and edge for the other side
(content/exe4/chips/topman/top.luau): a side-1 top leaving to the left went in a puff on column 0 where a side-0
top goes on unseen.

### 1.11 EXE4's throw

EXE4's thrown bombs (0x080EB47A) leave right of the navi whatever its side: a side-1 navi's left 4 pixels behind
it and flew 8 pixels further, landing later. nettai's leave toward the other side's area as the canonical side's
do (`releases_by_side`, content/exelib/bombs/throw.luau).

### 1.12 The field objects' order (EXE5, EXE6)

`field.all_objects()` lists the field-object registry's slots in order: side 0's three, side 1's three, the stage's
two. Where the order counts, the original goes through side 0's objects first whatever the user's side: ColArmy's
soldiers update, and draw their shots' heights, in it (`sub_80C9EE6`); DustMan and CosmoMan keep what they take in
it and throw it back in it. `field.all_objects(side)` lists them as that side goes through them: the slot order for
the canonical side, the two sides' slots traded for the other (nettai-luau's binding).

### 1.13 Ties at the user's column (EXE5, EXE6)

AntiDmg's shuriken (@exelib/antidmg/counter, `flip_toward`) flies back when its target is left of the navi or in its
column: a side-0 navi's shuriken at a target in its column flies back, a side-1 navi's ahead. EXE5's Football
(0x080D957C) takes a block in its owner's column facing right and not facing left. Both go as the canonical side's.

### 1.14 Misread tables by side (EXE4, EXE5)

Two routines index a table of words by the side unscaled, so side 1 reads a word rotated out of the row: EXE4's
Rush (0x080CA6F0: side 1's navi must stand on a side-1 panel for him to come up under it, side 0's on any;
@exelib/supports/rush) and EXE4's and EXE5's VDoll curse (0x080E89BA, 0x080E7356: side 1's asks a flag no combatant
sets, so a side-1 curse sparkles over nobody; @exelib/vdoll/curse). The other side reads the canonical side's word,
mirrored (the side bits traded).

### 1.15 Lists built side 0 first (EXE4)

BugChain's marks (0x080E6624) spawn over side 0's navi first, and JunkSoul's list of the chips sent
(rules/sent_chips, 0x08008916) takes side 0's hand before side 1's on both consoles, which the screen's shuffle and
"the first of a class" read (navis/megaman/forms/junksoul/recycle). Both are in the canonical side's order for the
user's (or the console's) side.

### 1.16 Region orders facing right (EXE6)

DustCross Beast's scatter (`sub_80EF3F4`) lists the 3x3 block's panels facing right whatever the navi's side, then
shuffles from that order. The other side's list is the canonical side's mirrored
(navis/megaman/forms/dustcross_beast/scatter).

### 1.17 Rounding by the sign of X (EXE5, EXE6)

A thrown obstacle's aim (`sub_8018002`, EXE5's Poltergeist and EXE4's: kinds/obstacle.rs `aim_throw`) takes the
arctangent and shifts the X speed by the sign of the X; LarkMan's swoop (EXE5's 0x080DDB78) keeps its turn's angle as
16.16 and draws by its whole steps, dropping the fraction toward minus infinity, so a swoop turning the other way lags
a step behind the mirror. Both work the other side's in the mirror image.

### 1.18 Off the field (EXE6)

BblStar's star (`sub_80CC668`) takes its region back when its panel's word (x | y << 8) isn't the one it was last
hit on, which starts at 0: column 0 of row 0, where a side-1 star leaving the field to the left lands, so it takes
none back, and a side-0 star leaving to the right does (column 7). Its word is in the canonical side's frame.

### 1.19 The auto battle's nearest enemy (EXE4, EXE5)

`sub_8015EC6` (@exelib/auto_battle/panels, `row_of_nearest_enemy`) means the row of the enemy nearest the navi but
reads the first panel an enemy stands on in the field's order, the one furthest right whatever the side. It reads the
first in the navi's side's order.

## 2. The recordings it changes

Each change departs from the original, so a recording in which a changed branch ran on the other side than the
canonical (side 1, by default) stops matching where it shows. bn6battle-verify's trace-tests lists each as a known
deviation (`src/deviations.rs`, the reason naming the entry above), by the default build.

Measured 2026-10-17 by replaying every recording with each fix turned off in turn (the fix whose turning off moves
a recording's first difference is its cause). Of EXE6's chip lab's 6,625 recordings 912 now differ, of the 2022
rounds 118 of 189, of EXE5's lab 198 of 2,586 and of EXE4's 258 of 2,366; sixteen rounds of the golden traces
(which list no deviations: their floors went down instead, trace-tests' tests/traces.rs).

| fix | EXE6 lab | 2022 rounds | EXE5 lab | EXE4 lab |
|---|---|---|---|---|
| §1.1 the panel under a position | 876 | 114 | 170 | 172 |
| §1.7 a stage's boulder's flags | | | 12 | 49 |
| §1.5 panel lists taken in order | 18 | 2 | 5 | 12 |
| §1.11 EXE4's throw | | | | 18 |
| §1.9 the hook's flames | 10 | | 2 | |
| §1.6 aims | 2 | 1 | 3 | 2 |
| §1.8 BlzrdBal's roller | 3 | | | |
| §1.17 LarkMan's turn | | | 3 | |
| §1.10 the screen's edges | | | | 3 |
| §1.2 a move passing a point | 1 | | | |
| §1.4 the thunder ball's re-aim | | 1 | | |
| §1.15 BugChain's marks, JunkSoul's list | | | | 2 |
| two at once (ChargeMan's volcano rocks, BlizMan's roller) | 2 | | 3 | |

The others (§1.3, §1.12 to §1.14, §1.16, §1.18, §1.19) change no recording's first difference: no recording runs
them for side 1, or one of the above shows first. The WindRack and barrier scenarios on side 1
(`chips/0x050-windrack/barrier-side1`, `barrier-far-side1`) are in the chip lab's library but not yet in the lab;
recorded, `barrier-side1` differs from the original by §1.1 (side 0's navi isn't pushed).

## 3. Audited and side-fair as they are

The shapes looked for, in the content of the three games and the engine's Rust: comparisons of an X with a tie
(`>=`, `<=`), column scans in a fixed order where the first found counts, tables indexed by side, checks of
`alliance` (or `alliance ^ flip`) with constants by side, fixed screen-direction defaults, edges at a fixed X, and
the core's searches, movement, panel ownership and collision pairing.

- Every game's collision types are mirror images by side: the high byte's bit pairs (0x80 and 0x40, 0x20 and 0x10,
  0x08 and 0x04, 0x02 and 0x01) and the next byte's 0x40 and 0x20.
- The panel types by side (GoingRd and ComingRd's roads, the projectiles' panel changes) are mirror images; so are
  the EXE4 ice push table's rows and the stolen-area bookkeeping.
- The chips' constants by side (`if me.alliance == 0 then 6 else 1` and their kin: DiveMan, SlashMan, GroundMan,
  ColForce, ChargeMan, JudgeMan, SpoutMan, BlastMan, the Beast Out berserk, DustCross's junk ball, EXE5's Slasher
  and GyroMan, EXE4's DeltaRay, VideoMan, MetalMan, ThunderMan, PanlSht, SearchSoul, ColdMan's sort) are mirror
  images.
- The hit regions' offsets turn by the facing; whole-field regions and the pair tests run along the field in a
  fixed order, but each registration meets each other at most once a tick, whatever the order.

## 4. Allowed: fair by distribution

A uniform pick (one draw modulo a list's length) from a list in the field's order gives the mirror another panel
for the same draw, each panel as likely for either side. These keep the original's order (the mirror check's
`ALLOWED`):

- GroundCross's charged chips' second and third falling rocks (`sub_8012DB8`).
- GroundMan's rocks (EXE6's chip and his link navi's).
- MetrKnuk's fists (the first at an enemy, the others at the marked panels).
- A meteor on a neighbor of its target's panel (up, down, left, right).
- EXE5's RedFruit's hops.
- EXE4's BlakWing's wings: each falls down a perch's column picked at random, and EXE4 lists its perches left to
  right (0x080DD1BA).
- EXE4's Duo's fists (MetrKnuk's searches).
- EXE5's Phoenix's targets: 18 draws from fixed tables of the middle columns and the two beside them.
- EXE5's CacDanc's cactuses (a navi's panel, or one beside it on a coin flip).
- CornFsta's bursts (one of the panels around an enemy).
- The auto battle's steps (EXE4's dark soul, EXE5's DarkInvs, dark survival, Chaos Unison and the AI navis) and the
  Beast Out berserk's (EXE6): a step to a panel one draw picks from a list in the field's order. (The mirror check
  allows these in the recordings of such navis alone, `SCOPED`: a navi's place is no kind of its own.)

(The modulo's bias, a list's length over 2^31, is the same for either side's list.)

## 5. Left as they are

- **The volcanos' halves (EXE6).** A volcano panel erupts by the field's cycle: the right half's (side 1's area at
  the start) at 0x46, the left half's at 0x8C. Side 1's area erupts first. No order of the halves is the same for
  both sides without changing the stage's beat; left as it is.
- **A pause on both sides at once.** `sub_800A046` looks at side 0 first: with both players pressing START on one
  tick, the battle pauses for side 0's, who alone resumes it. One has to be first, as in the update order. (A
  host that answers the pause key itself, `HostKeys::pause`, hears both.)
- **Drawn things.** A hit spark's jitter, a body's shake in a dimming, GyroMan's strikes' dust (up to 15 pixels
  to the right of the panel's center by a draw, whatever the side; the strike hits by its panel), the attachments
  held at an identity's fixed offset (Gregar Beast Out's (11, −7), not mirrored), a navi parked off the field at X
  180 (ChargeCross's tackle, GroundCross's drill), the judge's rolling digits, the soul image's spiral, the
  sprites that look each way by an animation of their own (LarkMan's swoop, SpoutMan's drip shower), SlashMan's
  charged waves' Z fraction (their spawner's side halfword), Count's charged shot jittering its navi in place:
  drawn only.
- **Values nothing reads.** A bug code's argument that is the target type's table offset (`row * 8 + side * 4`,
  EXE6's codes 0xF7 and 0xF8, whose routines read no argument).
- **The NaviCust bug byte past a side's block (EXE5).** A code between the block's end and twice it writes side 1's
  stats when side 0's navi takes it, and nothing for side 1's. No chip of the three games inflicts such a code.
- **EXE4's local console.** Its counter strikes and LifeSync look at which side is the local console's, not at
  the object's side: a question of the console, not of the field.
