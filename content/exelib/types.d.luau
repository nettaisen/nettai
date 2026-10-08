-- The shared types of content/exelib's modules (docs/design/rules-in-luau.md): what a game passes them (its looks,
-- kinds and constants) and what they hand back. Every content folder's definitions check together, so EXE5's and
-- EXE6's modules use these as their own.

-- How an attachment (@exelib/attachment/attachment) animates: its animation, whether it animates while dimmed too,
-- and what is added to its palette (drawn only).
export type AttachmentOptions = { anim: number?, while_dimmed: boolean?, palette_add: number? }

-- A game's sonic boom (@exelib/swords/sonic_boom): its sprite, its height, its palette when it goes through, else by
-- its element (`element_mask` of its element byte indexing `palettes`), its collision types (what it is, and what it
-- hits).
export type SonicBoomLook = {
    sprite: SpriteAsset,
    z: number,
    piercing_palette: number,
    palettes: { [number]: number },
    element_mask: number,
    is: CollisionType,
    hits: CollisionType,
}

-- What a game's sword slash (@exelib/swords/slash) asks of the user's form: the blade's animation where a chip doesn't
-- say, and whether the user's side is in a form where a step sword neither steps nor steps back (EXE6's Beast forms:
-- its lib/swords/slash).
export type SlashGame = {
    blade_anim: ((Object) -> number)?,
    stays: ((side: number) -> boolean)?,
    -- The game's step (EXE5's by the attack's charge: lib/swords); none: EXE6's, by the chip's `step`.
    steps: SlashSteps?,
    -- How the navi holds the blade in its related slot, in animation `anim` (none: @exelib/attachment's kind with
    -- the blade's palette, EXE6's and EXE5's; EXE4's own attachment).
    hold: ((me: Object, blade: AttachmentLook, anim: number) -> ())?,
    -- The swing restarts a form's overlay with the navi (`sub_8011450`: EXE6's, EXE5's; none: true); EXE4's swing
    -- (0x080EB5FA) calls nothing of the kind.
    refreshes_form_overlay: boolean?,
    -- The recovery clears the navi's moving flag and lets go of the overlay's too (EXE6's `sub_80EBBE2`, EXE5's;
    -- none: true); EXE4's (0x080EB832) lets go of the blade alone.
    recovery_clears_moving: boolean?,
}

-- A game's buster parts (@exelib/buster): the muzzle flash a shot leaves, the buster arm, an enemy navi's arm, the
-- recovery after a shot (by Rapid, then the open panels ahead), and its attachment kind.
export type BusterLook = {
    flash: AttachmentLook,
    arm: AttachmentLook,
    navi_arm: AttachmentLook,
    recovery: { { number } },
    -- The panel flags that stop a shot's panel count, by the shooter's side (0 and 1).
    closed: { [number]: number },
    attachment: Kind,
}

-- What the buster's shots share, made of a game's `BusterLook` (@exelib/buster).
export type Buster = {
    flash: AttachmentLook,
    arm: AttachmentLook,
    raise_arm: (navi: Object) -> (),
    raise_arm_for: (navi: Object, holder: Object, which: Slot, while_dimmed: boolean) -> (),
    recovery: (navi: Object, x: number, y: number) -> number,
    move_cancel: (navi: Object) -> boolean,
}

-- Firing a game's projectile (@exelib/projectile): from any panel, or from the panel in front as the buster does (the
-- buster's recovery from there).
export type Projectiles = {
    spawn: (owner: Object, x: number, y: number, s: ProjectileShot) -> Object?,
    fire: (navi: Object, s: ProjectileShot) -> number,
}

-- A game's shooting chips (@exelib/cannon, @exelib/airshot): the shot's sound, how the navi holds what it fires and
-- its projectiles.
export type ShooterLook = {
    sound: Sound,
    -- The navi holds `look` in slot `slot`: EXE6's and EXE5's @exelib/attachment kind, its animation by the holder's
    -- AI index from the action's table (`attachment.holder`); EXE4's own attachment kind (0x080CD6E0), its animation
    -- the navi's number (NaviStats +0x23).
    hold: (me: Object, look: AttachmentLook, slot: Slot) -> (),
    projectiles: Projectiles,
    -- A cannon's afterimages after its two (@exelib/cannon; EXE5's Hub Style shade), by the navi's flip.
    more_afterimages: ((me: Object, flip: number) -> ())?,
}

-- A game's cannons (@exelib/cannon): a shooter's look and the navi's animations as it raises the cannon and lowers it
-- (EXE6's and EXE5's 8 and 7; EXE4's 9 and 8); a GigaCan's afterimages in place of the two (EXE4's 0x080EB8BE: the
-- navi's first, its cannon where the navi stands, then a soul's part), by the navi's flip.
export type CannonLook = ShooterLook & {
    raise_anim: number,
    lower_anim: number,
    afterimages: ((me: Object, spec: CannonSpec, flip: number) -> ())?,
}

-- A game's throw (@exelib/bombs/throw): the wind-up's sound; its attachment kind (what the navi holds) or how the
-- navi holds it, in its related slot in animation `anim` (EXE4's own attachment, and nothing in hand for a navi of
-- number 5 or 10); and whether the thrown thing leaves 4 pixels right of the navi whichever way it faces (EXE4's
-- 0x080EB47A; none: ahead of it, EXE6's and EXE5's).
export type ThrowLook = {
    sound: Sound,
    attachment: Kind,
    hold: ((me: Object, held: AttachmentLook, anim: number) -> ())?,
    releases_right: boolean?,
}

-- A game's thrown bomb (@exelib/bombs/bomb): its kind's key, its sprite, its landing's sound, the splash in a hole,
-- the explosion over its blast and the puff when the battle ends.
export type BombLook = { id: string, sprite: SpriteAsset, hit_sound: Sound, splash: Effect, explosion: Effect, puff: Effect }

-- A game's thrown bombs, made of its `BombLook` (@exelib/bombs/bomb): the kind, and a thrower of a variant.
export type Bombs = { kind: Kind, thrower: (variant: BombVariant) -> Thrower }

-- Aiming thrown things over one game's sine table (@exelib/trajectory).
export type Trajectory = {
    div: (num: number, denom: number) -> (number, number, number),
    sqrt: (x: number) -> number,
    arctan2: (x: number, y: number) -> number,
    sine: (i: number) -> number,
    angle: (dx: number, dy: number) -> number,
    toward: (angle: number, speed: number) -> (number, number),
    in_ticks: (pos: Vec3, x: number, y: number, z: number, ticks: number, gravity: number) -> Vec3,
    at_speed: (pos: Vec3, vel_y: number, x: number, y: number, z: number, speed: number, gravity: number) -> (number, number),
}

-- A game's wide wave (@exelib/widesht/wave): its kind's key, its sprite and sound, its afterimages' sprite (a
-- variant's own overrides both; a game with no trailing variant, EXE4, states none), and what it hits.
export type WideWaveLook = {
    id: string,
    sprite: SpriteAsset,
    sound: Sound,
    trail_sprite: SpriteAsset?,
    target: CollisionType,
}

-- A game's wide shots (@exelib/widesht/action): its wide shooter (a spec's own overrides it), its attachment kind and
-- its wave's kind.
export type WideShotLook = { shooter: AttachmentLook, attachment: Kind, wave: Kind }

-- A wide shot: its waves (`shooter` and `z`, where a chip's aren't its game's wide shooter and 16 pixels).
export type WideShotSpec = { id: string?, wave: WaveVariant, waves: number, shooter: AttachmentLook?, z: number? }

-- What a chip's vulcan fires (@exelib/vulcan/action): how many bullets, and which; the vulcan the navi holds and the
-- bullets' kind (none: the game's).
export type VulcanSpec = {
    -- The action's key when no chip definition holds it (its compat key).
    id: string?,
    shots: number,
    bullet: BulletVariant,
    arm: AttachmentLook?,
    bullet_kind: Kind?,
}

-- A game's vulcans (@exelib/vulcan/action): the shot's sound, the vulcan the navi holds, how it holds it and where
-- (EXE6's and EXE5's @exelib/attachment kind in the overlay; EXE4's own in the related slot, its animation three
-- times the navi's number), its bullets' kind, and the navi's animations as it holds the vulcan and as it fires
-- (EXE6's and EXE5's 0xA and 0xD; EXE4's 13 and 17).
export type VulcanLook = {
    sound: Sound,
    -- The gun a chip that names none holds (none: every chip names its own, EXE4's).
    arm: AttachmentLook?,
    hold: (me: Object, look: AttachmentLook, slot: Slot) -> (),
    slot: Slot,
    bullet_kind: Kind,
    hold_anim: number,
    fire_anim: number,
}

-- What a chip's blow raises (@exelib/tornado/blow): one tornado (`single`), or one on each panel of `spread` at its
-- level (the chip's `level`, capped at 3: Static's the navi's kinds of NaviCust bug, DarkTorn's its side's statistic 3;
-- `panels` by level, { dx, dy } from the panel two ahead, dx toward the navi's front; `tornadoes`, by level, over
-- `tornado`: tornadoes that differ by level, EXE5's Static's); and the fan it holds, with an addition to its palette.
export type TornadoBlow = {
    -- The action's key when no chip definition holds it.
    id: string?,
    fan: AttachmentLook,
    fan_palette: number?,
    -- The tornadoes' kind (none: the game's).
    tornado_kind: Kind?,
    single: TornadoSpec?,
    spread: {
        level: (me: Object) -> number,
        panels: { [number]: { { number } } },
        tornado: TornadoSpec,
        tornadoes: { [number]: TornadoSpec }?,
    }?,
}

-- A game's blows (@exelib/tornado/blow): its attachment kind (the fan's) and its tornadoes' kind.
export type TornadoBlowLook = { attachment: Kind, tornado: Kind }

-- A chip's thunder shot (@exelib/thunder/shoot): its ball; `id`, the action's key when no chip definition holds it;
-- `kind`, a ball kind of its own (none: the game's).
export type ThunderShot = { id: string?, ball: ThunderBall, kind: Kind? }

-- A game's thunder shots (@exelib/thunder/shoot): its buster (the arm it raises; none: EXE4's raises none), its
-- ball's kind, and the navi's animation as it shoots (none: EXE6's and EXE5's 0x0E; EXE4's 0x12).
export type ThunderShotLook = { buster: Buster?, ball: Kind, anim: number? }

-- How a falling meteor's panel marker blinks (@exelib/meteors/marker; EXE6's `byte_80E4334`, by the original's first
-- parameter): its lifetime, its blink period, the ticks lit each period, and how much each period shrinks the next
-- (doubling).
export type MeteorMarkerLook = {
    lifetime: number,
    period: number,
    lit: number,
    shrink: number,
}

-- A game's boomerang (@exelib/boomer/boomerang): its kind's key, its sprite and sound, what it is (where a variant
-- doesn't say) and hits, and its spark (where a variant doesn't say).
export type BoomerangLook = {
    id: string,
    sprite: SpriteAsset,
    sound: Sound,
    is: CollisionType,
    hits: CollisionType,
    spark: Spark,
}

-- A boomerang's variant: 16.16 pixels a tick along a row and along the
-- column, whether it turns the other side's panels it crosses to grass;
-- its sprite, collision type and spark where a game's kind's aren't its
-- (EXE5's Boomer, EXE6's TomahawkCross Beast's tomahawk); its shadow (none:
-- on the ground; the tomahawk's its sprite's own part) and its animation
-- off row 1 (none: 0; the tomahawk's 1).
export type BoomerangVariant = {
    speed: number,
    turn_speed: number,
    grass: boolean,
    shadow: Shadow?,
    off_row_1_anim: number?,
    sprite: SpriteAsset?,
    is: CollisionType?,
    spark: Spark?,
}

-- How a boomerang is thrown: its variant; the column (0 to 2) where it
-- turns, or `turns = false` (the original's Param2 3: it flies straight
-- along its row); a stronger hit (hit modifier 3, else 1).
export type BoomerangThrow = { variant: BoomerangVariant, turns: boolean?, column: number?, strong: boolean? }

-- A game's ProtoMan (@exelib/protoman/navi): his kind's key, the sounds of his coming and his swing, his attachment
-- kind (his sword's), and his slash's collision types (what it hits, and what it is).
export type ProtoManKind = {
    id: string,
    appear_sound: Sound,
    swing_sound: Sound,
    attachment: Kind,
    target: CollisionType,
    self_type: CollisionType,
    -- EXE4's (nil: EXE6's and EXE5's). His spawner (0x080C721A) keeps neither his user, nor its facing, nor the element,
    -- and his height is his side, which his init keeps.
    summoned_alone: boolean?,
    -- What a panel he lands on must be free of, by his side (0x0F80_0000 for both), and whether his user's panel counts
    -- as free anyway (true).
    occupied: { [number]: number }?,
    user_panel_free: boolean?,
    -- His slash (0x080C713E, 0x080C71AA): the sword, the hit and its effect as it starts (EXE6's and EXE5's hit 10
    -- ticks in), the sword away as it ends (theirs at his blink out and as he goes).
    slash_at_once: boolean?,
    -- How he holds his sword (none: @exelib/attachment's `attachment` kind).
    attach: ((owner: Object, look: AttachmentLook, which: Slot, options: AttachmentOptions?) -> Object?)?,
}

-- A game's MegaMan's shots (@exelib/megaman): the action's key, its buster parts, its projectiles, its attachment
-- kind, the shot's sound, and the projectile a shot fires without a setup's (the table's first row).
export type MegaManShotLook = {
    id: string,
    buster: Buster,
    projectiles: Projectiles,
    attachment: Kind,
    shot_sound: Sound,
    plain: ProjectileVariant,
    -- Whether the charged shot waits before the buster comes up, asked on its first tick (none: always, EXE6's; EXE5's
    -- in the own-gauges mode alone, marking the panel in front).
    waits: ((me: Object) -> boolean)?,
}

-- A game's rapid buster (@exelib/megaman/rapid_buster): its action's key, its buster parts, projectiles, attachment
-- kind and shot sound, the shot (the projectile table's row 6), and whether a restarted form overlay steps at once
-- (EXE6's `sub_80C44D2`; EXE5's has it reload its animation at its next step).
export type RapidBusterLook = {
    id: string,
    buster: Buster,
    projectiles: Projectiles,
    attachment: Kind,
    shot_sound: Sound,
    shot: ProjectileVariant,
    overlay_steps: boolean,
}

-- A game's buster shot (@exelib/megaman/buster): its shot's look, a throw's sound and its throw of the obstacle the
-- navi absorbed (none: the game throws none), and the spread's muzzle flash (none: the shot's; EXE5's own, its
-- attachment row 0x39).
export type MegaManBusterLook = MegaManShotLook & {
    spread_flash: AttachmentLook?,
}

-- A buster setup's throw (@exelib/megaman/buster's `throw`): what flies instead of the shot (its sound, then it).
export type MegaManBusterThrow = { run: (me: Object) -> () }

-- What a spreader fires (@exelib/spreadr/action): its gun (EXE6's attachment rows 0x15 and 0x2F, by `dword_80ECC54`),
-- the bullet's height (16.16, by `dword_80ECC5C`), and the bullet; its own bullet kind and shot sound (none: the
-- game's; EXE5's Spreader's); the count the bullet leaves after (none: 1; EXE5's by its variant, 0x080EC57C: its
-- HeatS's and BubSht's 10).
export type SpreaderSpec = {
    id: string?,
    gun: AttachmentLook,
    z: number,
    bullet: BulletVariant,
    bullet_kind: Kind?,
    sound: Sound?,
    fire_after: number?,
}

-- A game's spreaders (@exelib/spreadr/action): the shot's sound, how the navi holds the gun and where (EXE6's and
-- EXE5's @exelib/attachment kind in the overlay, its animation by the holder's AI index from the game's table,
-- `attachment.holder`; EXE4's own in the related slot, its animation twice the navi's number), its bullets' kind,
-- and the navi's animation as it fires (EXE6's and EXE5's 0xB; EXE4's 14).
export type SpreaderLook = {
    sound: Sound,
    hold: (me: Object, look: AttachmentLook, slot: Slot) -> (),
    slot: Slot,
    bullet_kind: Kind,
    anim: number,
}

-- A trap chip's object (a "trap" record, @exelib/traps/controller): `set` spawns it for the controller and its user,
-- with the chip and bonus the controller keeps.
export type Trap = {
    set: (controller: Object, user: Object, chip: Chip?, bonus: number) -> Object?,
}

-- A hockey puck's variant (@exelib/airhocky/puck: a row of EXE6's `byte_80C9818`, EXE5's 0x080CA3A4): the steps it
-- lasts (0: no limit), how it bounces (1: off its own side too, once across), the ticks per step, and whether it
-- leaves a trail.
export type PuckVariant = { steps: number, own_side: number, ticks: number, trail: boolean? }

-- How a gust blows (@exelib/gust/gust, the original's first parameter): `pulls` turns it back toward its side, a
-- weaker push; `unseen` hides it and lets it blow through any panel.
export type GustHow = { pulls: boolean?, unseen: boolean? }

-- A game's gust (@exelib/gust/gust): its sprite, what it is and reaches, the hit flags that stop it (EXE6's wind
-- stoppers; EXE5's none, 0), whether an unseen gust pushes weaker too (EXE5's: any first parameter), and the panels it
-- blows over by side (require, forbid; side 2 reads the code after the table).
export type GustLook = {
    sprite: SpriteAsset,
    is: CollisionType,
    hits: CollisionType,
    stopped_by: number,
    weak_when_unseen: boolean,
    panels: { [number]: { number } },
}

-- A game's hockey puck: its sprite (the trail's too), its sound and the bounce's, what it is and reaches, its hits'
-- spark, the effect it bursts in, and its trail's tint.
export type PuckLook = {
    sprite: SpriteAsset,
    sound: Sound,
    bounce: Sound,
    is: CollisionType,
    hits: CollisionType,
    spark: Spark,
    burst: Effect,
    trail_tint: number?,
}

-- A game's magnet field (@exelib/magbolt/magnet): its key, what it is and hits, its spark by its element, the paralysis
-- it carries when its parameter says, and whether it only watches its owner's first related object (EXE4's) rather
-- than being stored there (EXE5's).
export type MagnetLook = {
    id: string,
    attack: CollisionType,
    hits: CollisionType,
    spark_of: (element: number) -> Spark,
    paralysis: Status,
    watches: boolean,
}

-- A game's magnet field's spawn: `owner`'s, at (dx, dy) from his panel, of an element and a damage word, paralyzing or
-- not.
export type MagnetSpawn = (
    owner: Object,
    dx: number,
    dy: number,
    element: number,
    paralyzes: boolean,
    damage: number
) -> Object?

-- A hockey chip's flick (@exelib/airhocky/flick): the action's key, its puck's kind and variant, whether it goes down
-- the field, and the navi's animation as it flicks (none: EXE6's and EXE5's 0x0C).
export type FlickSpec = { id: string?, kind: Kind, puck: PuckVariant, down: boolean, anim: number? }

-- A sun beam's look (@exelib/gundels/beam): its sprite and palette.
export type SunBeamLook = { sprite: SpriteAsset, palette: number }

-- A game's BugBomb (@exelib/bugbomb/bomb): its sprite and animation, what it is and is hit by while it sits (and
-- its hits' status), what it bursts as and reaches; the explosions (a crushed one's and the burst's on each panel
-- of the 3x3), the sounds of its landing, its crushing and its burst; the bug its burst gives (setting the
-- collision's bugs from its target's stats); the bubble it leaves in a hole (a rising bubble's kind) and its
-- trajectory.
export type BugBombLook = {
    sprite: SpriteAsset,
    anim: number,
    is: CollisionType,
    hit_by: CollisionType,
    status: Status,
    burst_is: CollisionType,
    burst_hits: CollisionType,
    crushed: Effect,
    burst: Effect,
    land_sound: Sound,
    crushed_sound: Sound,
    burst_sound: Sound,
    pick_bug: (me: Object) -> (),
    bubble: Kind,
    trajectory: Trajectory,
}

-- A game's snake (@exelib/snake/snake): its sprite and sounds, the ticks it waits before its leap, its strike's
-- element, spark, hit modifier and collision types, how it tells its nest it's out (`joined`) and gone (`left`), and
-- its trajectory.
export type SnakeLook = {
    trajectory: Trajectory,
    sprite: SpriteAsset,
    rise_sound: Sound,
    leap_sound: Sound,
    wait: number,
    strike_element: number,
    strike_spark: Spark,
    strike_hit_mod: number,
    is: CollisionType,
    hits: CollisionType,
    joined: (me: Object) -> (),
    left: (me: Object) -> (),
}

-- A snake's state: the panel it leaps at (x 0xFF: none), its nest, and its place in its nest's flags (EXE5's).
export type SnakeState = { target_x: number, target_y: number, nest: Object?, slot: number }

-- A nest's scan: where it is (bytes, the column stepping off the field when the scan ends).
export type SnakeScan = { scan_x: number, scan_y: number }

-- A game's tank shell (@exelib/tankcan/shell): what it is and hits, its sounds (the shot, a hit or blast), the
-- edge blast's look and collision type, the explosions' and a hit's burst's effects.
export type TankShellLook = {
    is: CollisionType,
    hits: CollisionType,
    sound: Sound,
    hit_sound: Sound,
    blast: Effect,
    blast_is: CollisionType,
    explosion: Effect,
    burst: Effect,
}

-- A chip's tank cannon (@exelib/tankcan/action): the action's key, the cannon it holds and its game's attachment
-- kind, the shell kind and how its shell flies (`ShellSpec`: EXE6's types).
export type TankCanSpec = { id: string?, cannon: AttachmentLook, attachment: Kind, kind: Kind, shell: ShellSpec }

-- A yoyo throw (@exelib/yoyo/throw): the action's key, the yoyo arm and its game's attachment kind, and what it
-- throws (from (x, y), `z` up, with the damage word).
export type YoyoThrowSpec = {
    id: string?,
    arm: AttachmentLook,
    attachment: Kind,
    throw: (me: Object, x: number, y: number, z: number, damage: number) -> Object?,
}

-- A game's moon blade (@exelib/moonbld/blade): its sprite, and what it is and hits.
export type MoonBladeLook = { sprite: SpriteAsset, is: CollisionType, hits: CollisionType }

-- A game's thrown seed (@exelib/bombs/seed): its sprite and animation, what it is and hits and its hit modifier, the
-- spark of its element its hits show (none: no spark), and the splash it leaves in a hole and the puff when the
-- battle ends.
export type SeedLook = {
    sprite: SpriteAsset,
    anim: number,
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    spark: ((element: number) -> Spark)?,
    splash: Effect,
    puff: Effect,
}

-- A game's H-Burst shot (@exelib/h_burst/burst): its probe's self type, what its bursts are and reach, their status
-- and the explosion on each of their panels.
export type HyperBurstShotLook = {
    probe: CollisionType,
    is: CollisionType,
    hits: CollisionType,
    status: Status,
    explosion: Effect,
}

-- A game's H-Burst (@exelib/h_burst/action): the spreader the navi holds, its attachment kind, the shot's sound and
-- the shot's kind.
export type HyperBurstLook = { spreader: AttachmentLook, attachment: Kind, sound: Sound, burst: Kind }

-- An H-Burst (EXE6's action 0x34, EXE5's 0x35): the bursts its shot makes where it hits.
export type HyperBurstSpec = { id: string?, bursts: number }

-- A game's BodyGrd striker (@exelib/bodygrd/striker): its kind's key, its identity, the shurikens' kind and the
-- throw's sound.
export type BodyGuardStrikerLook = { id: string, identity: Identity, shuriken: Kind, throw_sound: Sound }

-- BassAnly's dark ball (@exelib/bassanly/shot): its slant (-1 up, 1
-- down), and how it homes.
export type DarkBallHoming = "none" | "diagonal" | "seeking"

export type DarkBall = {
    slant: number,
    homing: DarkBallHoming,
}

-- A game's immobilizer (@exelib/instant/immobilizer): the mark it shows
-- (none: EXE6's, unseen) and its animation, its sound, the status its hit
-- carries and its collision types.
export type ImmobilizerLook = {
    sprite: SpriteAsset?,
    anim: number?,
    sound: Sound,
    status: Status,
    target: CollisionType,
    self_type: CollisionType,
}

-- A game's step of its swords' slash (@exelib/swords/slash; none: EXE6's, by
-- the chip's `step`): whether a slash steps (its afterimages on the swing,
-- its step back after it), where to (`set_off`: the move's start, refused
-- while moving or sliding; "step" two panels ahead, "warp" behind the
-- enemy, turned round; nil none), and the panel a warp lands on (nil:
-- none).
export type SlashSteps = {
    steps: (me: Object) -> boolean,
    kind: (me: Object, set_off: (Object) -> boolean) -> ("step" | "warp")?,
    warp_target: (me: Object) -> (number?, number?),
}

-- A NaviCust program's effect (@exelib/navicust/effects' constructors make them; @exelib/navicust/compile applies
-- them): its kind, and the kind's fields.
export type NaviCustEffect = {
    kind: string,
    amount: number?,
    weapon: Weapon?,
    barrier: Barrier?,
    what: string?,
}

-- A game's NaviCust board (its rules/navicust/board.luau): the grid's size, its command line, the board by expansions
-- (`o` a cell, `f` its frame, `.` none), and where a neighbor counts (`x` a cell where it doesn't).
export type NaviCustBoard = {
    SIZE: number,
    COMMAND_LINE: number,
    boards: { { string } },
    NEIGHBORS: { string },
}

-- A program on a side's NaviCust, an entry of its setup's `navicust_programs` (EXE6's and EXE5's rules/init): which,
-- in which of its colors, its center on the 7x7 grid, turned a quarter clockwise `rotation` times, compressed or not.
export type PlacedProgram = {
    program: NaviCustProgram,
    color: string,
    x: number,
    y: number,
    rotation: number,
    compressed: boolean,
}

-- What a game's NaviCust compile (@exelib/navicust/compile) is made of besides the shared routines: its name (for
-- errors), its board, its bugs in the order the bugs' routine runs them and what each writes by its level (1 to 3;
-- `hp`, what the HP programs add), what a placed program counts besides (EXE5's HubBatc), whether any bug sets the
-- emotion window's glitch, and what stops the bugs or keeps them whatever BugStop says (`bug_stop`: EXE5's patch
-- cards; nil, nothing).
export type NaviCustGame = {
    name: string,
    board: NaviCustBoard,
    bugs: { string },
    bug_work: { [string]: (stats: Navi, level: number, hp: number) -> () },
    extra: ((part: PlacedProgram, count: (bug: string?, n: number?) -> ()) -> ())?,
    glitch: boolean,
    bug_stop: ((side: number) -> boolean?)?,
}

-- What AirSpin sends from the panel ahead (@exelib/airspin/action): the
-- whirlwind top that rolls and spins there (its palette, its spins), EXE6's
-- seeking whirlwind no chip sends, or EXE5's AqWhirl whirlpool.
export type AirSpinSender = {
    send: (user: Object, x: number, y: number, element: number, damage: number) -> Object?,
    palette: number?,
    spins: number?,
}

-- AirSpin (@exelib/airspin/action: EXE6's action 0x38, EXE5's 0x47).
-- A game's seeking whirlwind (@exelib/airspin/whirl): its kind's id, its sheet, the sound it comes with, what it is
-- and reaches, and the statuses that hide an enemy navi from it (EXE6's invisible and submerged; EXE5's bubbled too).
export type SeekingWhirlLook = {
    id: string,
    sprite: SpriteAsset,
    sound: Sound,
    attack: CollisionType,
    hits: CollisionType,
    hidden: { StatusFlag },
}

export type AirSpinSpec = {
    id: string?,
    whirlwind: AirSpinSender,
}

-- A game's whirlwind top (@exelib/airspin/top): its kind's id, sprite,
-- identity and HP; its collision (its body, what hits it); its gust (the
-- effect, its palette offset, whether it faces its side's way, the hit's
-- type and targets); the effects and sounds; and the games' differences:
-- EXE6's set-down check, wind hits' extra spins and removal by chips (the
-- absorbed look), EXE5's markers and spins counted on Timer 2.
export type AirSpinTopLook = {
    id: string,
    sprite: SpriteAsset,
    identity: Identity,
    hp: number,
    body: CollisionType,
    hit_by: CollisionType,
    gust: Effect,
    gust_palette: number,
    gust_side: boolean,
    gust_self: CollisionType,
    gust_target: CollisionType,
    puff: Effect,
    explosion: Effect,
    sounds: { set_down: Sound, spin: Sound, hit_bomb: Sound },
    set_down_check: boolean,
    extra_spins: boolean,
    absorbed: AbsorbedLook?,
    marker: ((owner: Object, x: number, y: number, ticks: number, style: number) -> ())?,
    spins_on_timer2: boolean,
}

-- A whirlwind top made for a game (@exelib/airspin/top).
export type AirSpinTop = {
    kind: Kind,
    spawn: (owner: Object, x: number, y: number, element: number, palette: number, spins: number, damage: number) -> Object?,
    sender: (palette: number, spins: number) -> AirSpinSender,
}

-- A game's Z Saver (@exelib/zsaver/action): its action's id; its sounds
-- (the swing, the command's); its blade (the attachment kind, the look,
-- its animation by its holder); the slashes' hit types and their effects
-- (by index, 1 to 3); how it throws the fourth slash's boom; and EXE5's
-- differences: the Katanas' step (its lib/stepsword), and the command's
-- window shut once A isn't held.
export type ZSaverLook = {
    id: string?,
    swing_sound: Sound,
    command_sound: Sound,
    attachment: Kind,
    blade: AttachmentLook,
    blade_anim: (me: Object) -> number,
    is: CollisionType,
    hits: CollisionType,
    effects: { Effect },
    boom: (owner: Object, x: number, y: number, element: number, damage: number, boom: SonicBoom) -> Object?,
    step: StepSword?,
    a_held_window: boolean,
}

-- EXE5's step before a charged sword (exe5 lib/stepsword), as Z Saver takes
-- it.
export type StepSword = {
    start: (me: Object, s: any, moving: boolean) -> (),
    afterimage: (me: Object, anim: number) -> (),
    flip: (me: Object) -> number,
    step_back: (me: Object, s: any) -> (),
}

-- A game's NumbrBl ball (@exelib/numbrbl/ball): its kind's id, sprite, the
-- ticks it sits, its sounds (set down, hitting), its explosion, its hit's
-- types, and the parts EXE5's hides as it waits and rolls.
export type NumberBallLook = {
    id: string,
    sprite: SpriteAsset,
    sit_ticks: number,
    set_sound: Sound,
    hit_sound: Sound,
    explosion: Effect,
    is: CollisionType,
    hits: CollisionType,
    hidden_parts: number?,
}

-- A NumbrBl ball made for a game (@exelib/numbrbl/ball).
export type NumberBall = {
    kind: Kind,
    spawn: (thrower: Object, x: number, y: number, z: number, wait: number) -> Object?,
}

-- A patch card's effect (@exelib/patch_cards/effects' constructors make them; a game's rules/patch_cards applies
-- them, @exelib/patch_cards/apply): its kind, and the kind's fields.
export type PatchCardEffect = {
    kind: string,
    amount: number?,
    element: ElementName?,
    weapon: Weapon?,
    variant: ProjectileVariant?,
    barrier: Barrier?,
    gauge: string?,
    on: boolean?,
    trail: string?,
    level: number?,
    blanks: number?,
    charged: number?,
    status: string?,
    what: string?,
    -- The card shows it as a bug.
    bug: boolean?,
    -- The group its card screen lists it in, as the effect's constructor says: "parameter" (a stat's change) or
    -- "ability" (the rest, the bugs among them).
    group: ("parameter" | "ability")?,
    -- A pair's half (EXE4's, kind "pair"): its `effects`, which apply only while the card `card` (its key) sits in
    -- slot `slot`.
    pair: { slot: number, card: string, effects: { PatchCardEffect } }?,
}

-- What a game's patch cards' application (@exelib/patch_cards/apply) reads of the game: its kinds in the order they
-- apply (the original's effect numbers), the kinds whose choice is part of what the effect is (and the choice's
-- field), the element bytes of the bodies and the hit statuses' bytes.
export type PatchCardsGame = {
    rank: { [string]: number },
    choice: { [string]: string },
    bodies: { [string]: number },
    hit_status: { [string]: number },
}

-- CopyDmg's action for a game (@exelib/copydmg/action): its id, the shot's
-- sound, how the navi raises its buster, and what it fires (the game's
-- mark).
export type CopyDamageSpec = {
    id: string?,
    sound: Sound,
    raise_arm: (me: Object) -> (),
    shot: (owner: Object, x: number, y: number, element: number, damage: number) -> Object?,
}

-- A game's CopyDmg mark (@exelib/copydmg/mark): its kind's id, sprite and
-- animation, its hit's sound and types; EXE5's reloads its animation at its
-- init and hits its panel each tick it marks in the own-gauges mode
-- (the hit's spark and types).
export type CopyDamageMarkLook = {
    id: string,
    sprite: SpriteAsset,
    anim: number,
    sound: Sound,
    is: CollisionType,
    hits: CollisionType,
    reload_anim: boolean,
    own_gauges_hit: { spark: Spark, is: CollisionType, hits: CollisionType }?,
}

-- A CopyDmg mark made for a game (@exelib/copydmg/mark).
export type CopyDamageMark = {
    kind: Kind,
    spawn: (owner: Object, x: number, y: number, element: number, damage: number) -> Object?,
}

-- A Wind's or Fan's fan (@exelib/wind/fan): its palette; `pulls`: it blows
-- Fan's pulling gust from the far column rather than Wind's pushing one
-- from the enemy's front column; ticks from becoming its side's wind to
-- the first gust; HP; its identity (what it is taken for) and how it looks
-- absorbed (objects/absorbed_obstacle's `look`; EXE6's, which absorbs it).
export type FanVariant = {
    palette: number,
    pulls: boolean,
    wait: number,
    hp: number,
    -- Its NameID's identity (none: EXE4's init writes no NameID, 0x080D64A8).
    identity: Identity?,
    absorbed: AbsorbedLook?,
}

-- A game's Wind and Fan fan (@exelib/wind/fan): its kind's id, look, sounds and effects, its collision types, the
-- game's gust, and EXE5's differences.
export type FanLook = {
    id: string,
    sprite: SpriteAsset,
    -- As it is placed, as it starts blowing, as it breaks.
    appear_sound: Sound,
    blow_sound: Sound,
    break_sound: Sound,
    -- Removed (a puff 12 pixels up; none where no removal puffs); broken (an explosion).
    puff: Effect?,
    explosion: Effect,
    -- What it is and what reaches it: its side's obstacles' (the other side's attacks, objects and bodies, and
    -- neutral objects).
    is: CollisionType,
    hits: CollisionType,
    -- The game's gust (`sub_80CD488`).
    spawn_gust: (x: number, y: number, alliance: number, flip: number, how: GustHow, damage: number, holder: Object?,
        slot: number) -> Object?,
    -- EXE6's: a chip's removal can make it blink out or be absorbed (`sub_800F8CE`, `sub_800F90E`); EXE5's leaving
    -- (0x080CE90E) has neither.
    absorbs: boolean,
    -- EXE5's Fan's gust (0x080CE96C) starts on the far column alone, none when another obstacle stands there; EXE6's
    -- tries the panels toward the fan.
    far_column_only: boolean?,
    -- Whatever removes it, it leaves as when broken: its sound and its explosion (EXE4's 0x080D664C).
    removal_explodes: boolean?,
    -- The column its gust starts on in `row` (0: none), the game's own where it differs from EXE6's and EXE5's
    -- (EXE4's 0x080D6688).
    column: ((me: Object, row: number, pulls: boolean) -> number)?,
}

export type Fan = {
    kind: Kind,
    -- `sub_80CD2B4`: a fan of `variant` on (x, y), of `owner`'s side and facing, running while dimmed, with the
    -- damage word `damage`; its side's one field object of class 1 (a second replaces the first). Its position is
    -- the spawner's registers until its init places it.
    spawn: (owner: Object, variant: FanVariant, x: number, y: number, damage: number) -> Object?,
}

-- A game's Wind and Fan controller (@exelib/wind/controller): its kind's id, the game's fan (`sub_80CD2B4`), and
-- where it spawns, by its user and the dimming (none: where `dimming_chips.spawn` puts it).
export type WindControllerLook = {
    id: string,
    spawn_fan: (owner: Object, variant: FanVariant, x: number, y: number, damage: number) -> Object?,
    position: ((user: Object, spec: DimmingChipSpec) -> Vec3)?,
    -- Its spawn copies the user's side alone, not its facing (EXE4's, 0x080E4B60).
    side_only: boolean?,
}

export type WindController = {
    kind: Kind,
    -- A chip's `dimming` hook (EXE6's `sub_80E3128`, EXE5's 0x080E32E4): the controller placing a fan of `variant`.
    hook: (variant: FanVariant) -> (user: Object, spec: DimmingChipSpec) -> Object?,
}

-- Where an owner keeps an attached object: its actor overlay slot, its
-- first related object, or its content state's `held` or `held2` field
-- (@exelib/slot).
export type Slot = "overlay" | "related" | "held" | "held2"

-- What a projectile's hit does to the panel it hits: crack it
-- (`object_crackPanel`), break it (`object_breakPanel_dup2`), or make a
-- solid one a panel of a type (by the shot's side).
export type PanelHit = {
    effect: "crack" | "break" | "set_type",
    left_side: PanelType?,
    right_side: PanelType?,
}

-- A projectile's variant (objects/projectile, the original's `off_80C4C78`
-- rows): what it is and hits, its hit modifier, its element and secondary
-- elements, its hit spark (nil: none), its look (none: not drawn), the
-- status and bug its hits carry (nil or 0: none), what its hit does
-- to the panel, whether it bursts (around what it hits, and over the last
-- two columns when it leaves the field), whether it climbs (a pixel lower
-- down the field and higher on each panel). One a navi's stats can name
-- (a shot program) has a key.
export type ProjectileVariant = {
    id: string?,
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    element: string,
    secondary: { string }?,
    spark: Spark?,
    sprite: SpriteAsset?,
    anim: number?,
    status: Status?,
    bug: number?,
    bug_arg: number?,
    hit_panel: PanelHit?,
    bursts: boolean?,
    climbs: boolean?,
}

-- A bullet's variant (objects/bullet, the original's `byte_80C68D4` rows):
-- the region its burst hits on the panel it found, the burst's hit
-- modifier and status (nil: none), the effect it shows (over
-- the region, or once at the bullet when `single`), and its sound.
export type BulletVariant = {
    region: Region,
    hit_mod: number,
    effect: Effect,
    single: boolean?,
    status: Status?,
    sound: Sound?,
}

-- How a bullet (objects/bullet) probes and hits: its self type while it
-- looks, its burst's, and what both reach.
export type BulletLook = {
    probe: CollisionType,
    attack: CollisionType,
    target: CollisionType,
    -- A single burst's effect shows unturned, whichever way the bullet faces (EXE4's).
    single_unturned: boolean?,
}

-- A cannon (lib/cannon.luau, the original's action 0x14): the arm the navi
-- holds, the projectile it fires, and whether it leaves afterimages as it
-- charges (the GigaCans).
export type CannonSpec = {
    -- The action's key when no chip definition holds it (its compat key).
    id: string?,
    arm: AttachmentLook,
    shot: ProjectileVariant,
    afterimages: boolean?,
}

-- A game's ProtoMan (chips/protoman/navi.luau, `protoman.look`): his
-- sprite and palette, his sword's look and animation, his slash's effect,
-- and his spawner's address (which his position takes).
export type ProtoManLook = {
    sprite: SpriteAsset,
    palette: number,
    sword: AttachmentLook,
    sword_anim: number,
    slash: Effect,
    spawner_address: number,
}

-- An AirShot (lib/airshot.luau, the original's action 0x21): the shooter
-- the navi holds and the projectile it fires.
export type AirShotSpec = {
    -- The action's key when no chip definition holds it.
    id: string?,
    shooter: AttachmentLook,
    shot: ProjectileVariant,
    -- The recovery's count (ticks less one): EXE6's and EXE5's 10 (`sub_80EC90E`); EXE4's the attack's variant, 30
    -- when that isn't 0 (0x080ECC02: AirShot's variant 0, 0).
    recovery: number,
}

-- A game's AirShot (@exelib/airshot): its shooter's look, the navi's animation as it fires (EXE6's and EXE5's 9,
-- EXE4's 12) and the slot the shooter takes (EXE6's and EXE5's overlay, EXE4's related).
export type AirShotLook = ShooterLook & { anim: number, slot: Slot }

-- A wide wave (chips/widesht/wave.luau, the original's attack #0x3B by
-- kind): its speed (16.16 pixels a tick); whether it goes through what it
-- hits (with the hit modifier it's fired with), holds its spawner's attack
-- and waits before sliding, or trails afterimages; what it is (a collision
-- type), the bug it gives and its palette.
export type WaveVariant = {
    speed: number,
    pierces: boolean,
    holds: boolean,
    trails: boolean,
    collision: CollisionType,
    bug: number?,
    palette: number?,
    -- Its sprite where a game's isn't EXE6's wide wave (EXE5's), and its
    -- trail's where that isn't EXE6's rock cubes (EXE5's is its wave's).
    sprite: SpriteAsset?,
    trail_sprite: SpriteAsset?,
}

-- A sonic boom (lib/swords/sonic_boom.luau): its hit modifier, whether it
-- goes through what it hits.
export type SonicBoom = { hit_mod: number, pierces: boolean }

-- TankCan's shell (chips/tankcan/shell.luau): its hit's modifier, whether
-- its blast at the field's edge cracks the panels, how far the blast's
-- damage word shifts right.
export type ShellSpec = { hit_mod: number, cracks: boolean, shift: number }

-- What a projectile (objects/projectile) is fired with
-- (lib/projectile.luau).
export type ProjectileShot = {
    -- What it is.
    variant: ProjectileVariant,
    -- The damage word: the damage (with its flag bits) | the counter
    -- byte << 16.
    damage: number,
    -- Its height, 16.16 (the buster's shots fly at 24 pixels, 0x18_0000).
    z: number,
    -- Rows it also moves on its first step: 1 down, -1 up (its second
    -- parameter). None by default.
    row: number?,
}

-- An attachment's look (objects/attachment): its sprite, its palette, the
-- pixels it sits raised, the owner's attach point it sits at (none: on the
-- owner's position); `by_owner`: at a fixed offset by the owner's NameID
-- instead (the original's row 0xF).
export type AttachmentLook = {
    sprite: SpriteAsset,
    palette: number,
    lift: number?,
    attach_point: number?,
    by_owner: boolean?,
}

-- The bombs and seeds (lib/bombs, docs/design/content-model-v2.md §5.1).
-- A thrower throws its thing from `pos` with the attack's damage word;
-- `variant` is what it throws (kept as data, so the canonical tree shows
-- what each chip throws).
export type Thrower = {
    throw: (user: Object, pos: Vec3, damage: number) -> Object?,
    variant: any?,
}

-- A throw (lib/bombs/throw): what the navi holds while winding up (its
-- look, animation and palette offset) and what throws it.
export type ThrowSpec = {
    -- The action's key when no chip definition holds it (its compat key).
    id: string?,
    held: AttachmentLook,
    held_anim: number?,
    held_palette: number?,
    thrower: Thrower,
}

-- A thrown bomb (lib/bombs/bomb): its collision, hit modifier, palette and
-- animation; the region it blasts where it lands (none: no blast); what it
-- leaves the tick after.
export type BombVariant = {
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    palette: number,
    anim: number,
    blast: Region?,
    -- Its own explosion over the blast, else the look's (EXE4's variants by
    -- 0x080CE277: DarkBomb's).
    explosion: Effect?,
    after: ((me: Object) -> ())?,
}

-- A thrown seed (lib/bombs/seed): its element and palette, the panels it
-- grows, the effect on each and its sound.
export type SeedVariant = {
    element: number,
    palette: number,
    panel: PanelType,
    effect: Effect,
    sound: Sound,
}

-- A sword's hit (lib/swords): its region on the panel ahead, what it is and
-- hits, its hit modifier, and the status and bug it inflicts (none by
-- default).
export type SwordHit = {
    region: Region,
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    status: Status?,
    bug: number?,
    bug_arg: number?,
}

-- A sword's slash (lib/swords/slash, the original's action 0x13): the blade
-- it holds, its hit and the slash drawn over the panel ahead (with a
-- palette offset), its swing's sound; DblDream swings twice, CrosSwrd adds
-- a single-panel hit, StepSwrd steps two panels ahead first.
export type SlashSpec = {
    -- The action's key when no chip definition holds it (its compat key).
    id: string?,
    blade: AttachmentLook,
    -- The blade's animation, by its holder (none: `sub_80EBAE8`'s,
    -- lib/swords/parts).
    blade_anim: ((Object) -> number)?,
    hit: SwordHit,
    effect: Effect,
    effect_palette: number?,
    sound: Sound,
    swings: number?,
    second_hit: SwordHit?,
    step: boolean?,
    -- What SlashCross's charged slash makes of it (a step sword dashes in
    -- first).
    charged: ChargedSlash?,
}

-- What SlashCross's charged slash is, of one sword
-- (navis/megaman/forms/slashcross/slashes): the sword wave it sends,
-- and a second one unseen under it (CrosSwrd's); the blade the navi
-- raises (none: it swings its arm alone, without one); the swing's sound;
-- the slash that follows it (DblDream's second); and the lock-on mode the
-- Beast Out rush takes for it (rules/lockon).
export type ChargedSlash = {
    wave: SwordWaveVariant,
    hidden_wave: SwordWaveVariant?,
    blade: AttachmentLook?,
    sound: Sound,
    again: ChargedSlash?,
    lockon: Lockon,
}

-- A rock's variant (chips/rockcube/rock, a row of the original's
-- `byte_80CF934`): its standing animation, HP, element, the palette of the
-- debris it breaks into, the sound it breaks with, and its identity (what
-- it is taken for: a rock, an ice block).
export type RockVariant = {
    -- Its key (`rockcube/rock/cube`): stages name their rocks' variants.
    id: string?,
    anim: number,
    hp: number,
    element: string,
    debris_palette: number,
    break_sound: Sound,
    identity: Identity,
}

-- How a rock comes onto the field (chips/rockcube/rock, the original's
-- third parameter): rising out of its panel (its sprite's first animation), there
-- at once, falling from above, or placed when the round starts (standing
-- from the start, and running while the battle is paused until it stands).
export type RockEntrance = "rise" | "instant" | "fall" | "placed"

-- What a rock is spawned with (chips/rockcube/rock): its variant, the
-- class of field-object slot it takes (0, the default: one of its side's
-- two; 1: its side's one), and its entrance ("rise" by default).
export type RockSpec = {
    variant: RockVariant,
    class: number?,
    entrance: RockEntrance?,
}

-- ---- The dimming chips (chips/wind, chips/guardian, ...) -------------------

-- CircGun's gun (chips/circgun/gun; the original's chip parameters): the
-- ticks between its steps less one, its shots, and its look: the palette
-- (the look's number), what its shots hit as (`dword_80D690C` by the look:
-- look 0 a thrown attack, look 1 another attacker's) and the bug they carry
-- with its argument (`byte_80D68D8`: look 1's 0x54, 0x32; none: none).
export type CircGunVariant = {
    period: number,
    shots: number,
    palette: number,
    hit: CollisionType,
    bug: number?,
    bug_arg: number?,
}

-- An Anubis statue (chips/anubis/statue, `byte_80CF0C0`): its animation,
-- the ticks between its drains, HP, lifetime, and identity (what it is
-- taken for).
export type AnubisVariant = {
    anim: number,
    period: number,
    hp: number,
    lifetime: number,
    identity: Identity,
}

-- A panel change (objects/panel_changer, a row of the original's
-- `byte_80E272C`): which panels (all, the changer's row, the panel in
-- front of it, its own) meeting its side's condition become `panel` (on
-- side 1 `side1_panel` if set: the roads point the other way) after
-- `ticks`, showing as it meanwhile (every tick, or with `flicker` every
-- other four), with a tick sound every fourth tick and a sound as they
-- change (nil: none).
export type PanelChange = {
    panel: PanelType,
    side1_panel: PanelType?,
    panels: "all" | "row" | "front" | "own",
    flicker: boolean,
    ticks: number,
    sides: { [number]: PanelCondition },
    -- The sound as it starts (EXE4's changes to holes: 0x080E4390), every fourth tick, and as the panels change.
    start_sound: Sound?,
    tick_sound: Sound?,
    change_sound: Sound?,
}

-- A barrier (lib/barriers): `sub_801A7CC`'s row (how it behaves, its HP,
-- the damage a hit must reach to wear it, its timer (0xFFFF: none), the
-- element that breaks it), and its visual (`byte_80E0A14`'s look: sprite,
-- animation, palette, the owner's attach point it sits at, the sprite
-- parts it hides; LifeAur's aura hides part 6 but for the local MegaMan),
-- with its sounds as it rises (and grows back) and as it goes down.
export type Barrier = {
    -- The key a navi's stats name it by (its first barrier).
    id: string?,
    behavior: BarrierBehavior,
    hp: number,
    threshold: number,
    timer: number,
    weak: string?,
    look: {
        sprite: SpriteAsset,
        anim: number,
        palette: number,
        attach_point: number,
        hidden_parts: number,
    },
    hidden_unless_local_megaman: boolean?,
    sound: Sound,
    down_sound: Sound?,
}

-- An instrument (lib/instruments, a row of the original's `byte_80D4078`):
-- its identity (what it is taken for) and palette,
-- how long it plays and pauses, its tune, and the effect it has every
-- third tick of playing (`off_80D4298`). Its HP and sprite where a game's
-- aren't the library's (60, `byte_80D4140`; EXE6's `instrument`): EXE5's
-- 100 and its own pack's; `absorbed` is `instrument.instrument`'s.
export type Instrument = {
    -- Its NameID's identity (none: EXE4's init writes no NameID, 0x080DC35C).
    identity: Identity?,
    palette: number,
    play_ticks: number,
    pause_ticks: number,
    tune: Sound,
    effect: (me: Object) -> (),
    hp: number?,
    sprite: SpriteAsset?,
    absorbed: AbsorbedLook?,
    -- Ticks between its effects (the row's byte 1; none: 3).
    effect_period: number?,
}

-- A sword wave (navis/megaman/forms/slashcross/sword_wave, a row of the
-- original's `byte_80D7F4C`): what it is and hits, its hit modifier and
-- region, its look, the panel centers it passes before it ends and its
-- speed (16.16 a tick, forward), the status and bug its hits carry
-- (none by default). `animates`: it restarts its animation after the last
-- frame and steps it; `highlight`: it lights the panels its region
-- covers; `ground_shadow`: its shadow stays on the ground; `steps`: its
-- sprite also steps after each update (the charged shot's own wave).
export type SwordWaveVariant = {
    collision: { is: CollisionType, hits: CollisionType },
    hit_mod: number,
    region: Region,
    sprite: SpriteAsset,
    anim: number,
    palette: number,
    reach: number,
    speed: number,
    status: Status?,
    bug: number?,
    bug_arg: number?,
    animates: boolean?,
    highlight: boolean?,
    ground_shadow: boolean?,
    steps: boolean?,
}

-- A countdown bomb (chips/timebom/countdown, a row of the original's
-- `byte_80CD8AC`): the animations it rises and stands with, its HP, its
-- NameID (the field objects' look, a number until identities come), the
-- class of field-object slot it takes (0: one of its side's two; 1: its
-- side's one), what it is and what it reacts to. `strong_shake`: its blast
-- shakes the camera a second time, harder (TimeBom+). The rows no chip
-- sets: `allows_bodies`, it may appear under a body; `bursts_when_broken`,
-- broken it blows up, and its countdown is only the last 15 ticks, after
-- which it blinks away; `linked`, its spawner keeps it in a slot and it
-- reserves its panel once it stands.
export type CountdownVariant = {
    appear_anim: number,
    idle_anim: number,
    hp: number,
    identity: Identity,
    class: number,
    collision: { is: CollisionType, hits: CollisionType },
    strong_shake: boolean?,
    allows_bodies: boolean?,
    bursts_when_broken: boolean?,
    linked: boolean?,
}

-- ColorPt's and DblPoint's points (chips/colorpt): their palette (four a
-- step) and the Atk+ bonus each brings (the chips' parameters).
-- (Its palette: the chip's, four times the original's first parameter.)
export type ColorPoint = { palette: number, bonus: number }

-- A tornado's look and rhythm (chips/tornado/tornado, a row of the
-- original's `byte_80CA064`): its palette, the ticks between hits, the bug
-- its hits carry (nil: none); `plain`: it takes its panel's variant.
export type TornadoVariant = { palette: number, rearm: number, bug: number?, plain: boolean? }

-- A game's tornado (chips/tornado/tornado, `make_kind`): the variant a
-- plain one takes on a panel type, whether that doubles its damage, its
-- self type (and the HP-bug variant's, when another), what it reaches and
-- the spark of its element.
export type TornadoLook = {
    sprite: SpriteAsset,
    sound: Sound,
    panel_variants: { [PanelType]: TornadoVariant },
    doubles_on_panel: boolean,
    is: CollisionType,
    bug_is: CollisionType?,
    target: CollisionType,
    spark: (element: number) -> Spark,
}

-- What a tornado is spawned with: its variant, the status its hits
-- inflict (0: none) and its ticks.
export type TornadoSpec = { variant: TornadoVariant, status: Status?, ticks: number }

-- A thunder ball (chips/thunder/ball): fast or not, the panels it drifts,
-- the status its hits inflict and their bug (none by default).
export type ThunderBall = {
    fast: boolean?,
    panels: number,
    status: Status?,
    bug: number?,
    -- Its chip's: big (DarkThnd's; a fast one is anyway), and its collision
    -- type and sound where they aren't the look's.
    big: boolean?,
    is: CollisionType?,
    sound: Sound?,
}

-- A game's thunder ball (chips/thunder/ball, `make_kind`): its sprite and
-- animation small and big (none: a game whose ball is never big, EXE4's),
-- its collision type, hit modifier and spark, its sound, and its own search
-- for its target (none: `sub_80C95EC`'s, EXE6's and EXE5's).
export type ThunderBallLook = {
    sprite: SpriteAsset,
    small_anim: number,
    big_sprite: SpriteAsset?,
    big_anim: number?,
    is: CollisionType,
    target: CollisionType,
    hit_mod: number,
    spark: Spark,
    sound: Sound,
    find_target: ((me: Object) -> (number, number))?,
}

-- ElemTrap's counterattack by element (chips/elemtrap/strike): its sound,
-- its hits' spark, hit modifier and status, and the bursts' look.
export type ElemTrapStrike = { sound: Sound?, spark: Spark, hit_mod: number, status: Status?, burst: Effect }

-- A game's counterattack (`strike.make_kind`): its strikes by element
-- (null to wood), what its hits are and reach, its bursts' kind; the panels
-- it strikes (none: `byte_80E36E4`'s mask over the field, EXE6's and EXE5's;
-- EXE4's the other side's navi's panels, 0x080E5244), a step after the hits
-- (EXE4's sets the other side's mood to 1, 0x0800F4DE), and where its bursts
-- go (none: the whole field; EXE4's the other side's area, 0x080E531F).
export type ElemTrapStrikeLook = {
    strikes: { [number]: ElemTrapStrike },
    self_type: CollisionType,
    target: CollisionType,
    bursts: Kind,
    targets: ((me: Object) -> { { x: number, y: number } })?,
    after: ((me: Object) -> ())?,
    bursts_over: ((me: Object) -> Region)?,
}

-- A game's trap (chips/elemtrap/trap, `trap.make_kind`): what springs it
-- (fire to wood's first hit, or one element's: the one it is set with),
-- what it is and reaches, where its sparkles show (16.16, from the
-- panel's center: down the field, up), and its counterattack's kind.
export type ElemTrapLook = {
    sparkle: Effect,
    sparkle_sound: Sound,
    watches: "fire_to_wood" | "one",
    self_type: CollisionType,
    target: CollisionType,
    sparkle_dy: number,
    sparkle_z: number,
    strike_kind: Kind,
}

-- How FireHit's fist strikes (chips/firehit/fist): the spark its hit shows
-- and its collision types.
export type FireHitFistLook = { sprite: SpriteAsset, sound: Sound, spark: Spark, target: CollisionType, self_type: CollisionType }

-- How a yoyo flies (chips/yoyo/yoyo): GreatYo's (three that move
-- together), the panels it rolls out and its spinning rounds.
export type YoyoSpec = { great: boolean?, panels: number, rounds: number }

-- A Reflector's shield's look (chips/rflectr/shield, a row of the
-- original's `byte_80C9664`): the sprite, its animation while up, the
-- animation and ticks of its fade, and its palette.
export type ReflectorShieldLook = { sprite: SpriteAsset, anim: number, fade_anim: number, fade_ticks: number, palette: number }

-- SpoutMan's geyser (chips/spoutman/geyser): whether it acts while the
-- battle is dimmed (the navi chip's), how long it erupts, and the column
-- its spawner aims it at.
export type Geyser = {
    while_dimmed: boolean?,
    ticks: number,
    target: number,
}

-- How a game's gun and shots look and sound: their sprite, the sounds of a
-- step, a mark and a hit, and where the gun and a shot sit (`on_floor`:
-- EXE5's, on the panel's center at height 0).
export type CircGunLook = {
    sprite: SpriteAsset,
    sounds: { step: Sound, mark: Sound, hit: Sound },
    -- What the hit reaches, and the flash it shows.
    target: CollisionType,
    flash: Effect,
    -- The hit's spark (element 0's).
    spark: Spark,
    on_floor: boolean?,
    -- Going back along a row it turns on a column with any of the user's side's panels (EXE4's, 0x080DE7AA), where
    -- EXE6's and EXE5's turn before a column with none of the other side's.
    turns_on_own_column: boolean?,
}

-- A game's CircGun controller's differences: its spawn takes the user's side alone, not its facing (EXE4's,
-- 0x080E8B84); it sets the gun whatever the far column holds (EXE4's 0x080E8B40, where EXE6's and EXE5's end at once
-- with none of the other side's panels there).
export type CircGunControllerLook = {
    side_only: boolean?,
    always_sets_gun: boolean?,
}

-- How a game's bomb looks and sounds, and how it leaves.
export type CountdownLook = {
    sprite: SpriteAsset,
    sounds: { place: Sound, tick: Sound, last: Sound, blast: Sound, puff: Sound },
    -- The blast's collision: what it is and what it hits.
    blast: { is: CollisionType, hits: CollisionType },
    -- How it looks absorbed (nil: nothing absorbs it).
    absorbed: AbsorbedLook?,
    -- Any removal leaves a puff (EXE5's): no absorbing, no blinking out.
    removal_puffs: boolean?,
    -- The blast's explosions and a removal's puff, and the panel bursts'
    -- kind.
    explosion: Effect,
    puff: Effect,
    bursts: Kind,
}

-- A lance's hit: what it is and hits, its spark, hit modifier and bug.
export type LanceHit = {
    is: CollisionType,
    hits: CollisionType,
    spark: Spark,
    hit_mod: number,
    bug: number?,
    bug_arg: number?,
}

-- How a game's lance looks: its sprite, its sound, and an attack the whole
-- time it shows (EXE4's `attack`; none: the chip's hit on its seventh tick,
-- EXE6's and EXE5's).
export type LanceLook = { sprite: SpriteAsset, sound: Sound, attack: LanceAttack? }

-- A lance that is an attack the whole time it shows (EXE4's, 0x080D73F8):
-- its collision registration from its init, what it is and hits and its
-- hit modifier, and its spark by its element; each tick its hits resolve
-- and spark, its first that lands ends its region, and its panel shows
-- highlighted while the region stands.
export type LanceAttack = { is: CollisionType, hits: CollisionType, hit_mod: number, spark: (element: number) -> Spark }

-- A row (`byte_80CF3AC`): what it is and reaches and its hit modifier; the
-- camera shake it lands with (0 none, else the magnitude, 15 ticks); what
-- it does to its panel (none, cracks it, cracks it on half the draws, or
-- breaks it); and the bug its hit carries (EXE5's DarkMetr's).
export type MeteorRow = {
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    shake: number,
    panel: "none" | "crack" | "crack_half" | "break",
    bug: number?,
    bug_arg: number?,
}

-- How a game's meteor looks and sounds, and its spark by element.
export type MeteorLook = {
    sprite: SpriteAsset,
    sounds: { fall: Sound, land: Sound },
    spark: (element: number) -> Spark,
    -- Its landing's burst, and its panel marker's kind.
    burst: Effect,
    marker: Kind,
    -- Where its hit is set as it lands: only where it bursts (EXE5's and EXE6's), or on its panel either way (EXE4's
    -- 0x080D873A).
    hit_on: "burst" | "landing",
    -- Whether its marker takes its side (EXE5's and EXE6's spawn sets it); EXE4's sets none, leaving side 0
    -- (0x080E5EEA).
    marker_takes_side: boolean,
}

-- How a game's drill is: its hit's spark, what the first and second are
-- and hit, and whether its spawn keeps the user's element and the second
-- takes the first's running while dimmed (EXE6's do; EXE5's 0x080D41C8 and
-- 0x080D4100 don't).
export type DrillLook = {
    spark: Spark,
    first: CollisionType,
    second: CollisionType,
    hits: CollisionType,
    keeps_element: boolean,
}

-- How a game's instruments are: their sprite and HP (an instrument's own
-- first), sounds, what they are and what reaches them, how they look
-- absorbed (nil: nothing absorbs them), and whether any removal just puffs
-- (no absorbing, no blinking out).
export type InstrumentLook = {
    sprite: SpriteAsset,
    hp: number,
    sounds: { appear: Sound, broken: Sound },
    body: { is: CollisionType, hits: CollisionType },
    absorbed: AbsorbedLook?,
    removal_puffs: boolean?,
    -- Whatever removes it, it leaves as when broken: its sound and its explosion (EXE4's 0x080DC58C).
    removal_explodes: boolean?,
    -- Broken, its explosion; removed, its puff (none where no removal puffs).
    explosion: Effect,
    puff: Effect?,
}

-- A game's Tango's heal (@exelib/supports/heal): its kind's key, sprite and sound, the sparkle as it lands, the game's
-- trajectory it arcs by, and how it raises the game's 100-HP barrier (type 5) and its visual on the navi.
export type TangoHealLook = {
    id: string,
    sprite: SpriteAsset,
    sound: Sound,
    sparkle: Effect,
    trajectory: Trajectory,
    raise_barrier: (navi: Object) -> (),
}

-- Tango's heal made for a game (@exelib/supports/heal): thrown by `tango` from `pos` onto the panel (x, y).
export type TangoHeal = {
    kind: Kind,
    spawn: (tango: Object, pos: Vec3, x: number, y: number) -> Object?,
}

-- A game's Tango (@exelib/supports/tango): her kind's key and sprite, her arrival's, landing's and throw's sounds, the
-- sparkle she appears in (effect 0x15), the puff she goes in (effect 0x14), and her heal; `pixel_nudge` false: she
-- starts on her navi's row, not a pixel down the field (EXE4's).
export type TangoLook = {
    id: string,
    sprite: SpriteAsset,
    arrive_sound: Sound,
    land_sound: Sound,
    throw_sound: Sound,
    arrive: Effect,
    puff: Effect,
    heal: TangoHeal,
    pixel_nudge: boolean?,
}

-- A game's Beat (@exelib/supports/beat): his kind's key and sprite, his swoop's and snatch's sounds.
export type BeatLook = {
    id: string,
    sprite: SpriteAsset,
    swoop_sound: Sound,
    snatch_sound: Sound,
}

-- A support made for a game (@exelib/supports/rush, beat, tango): sent by the supports' controller, whose `out` flag
-- it sets (Rush eating the chip the controller carries); and whether its host warps out while it comes and back in
-- after (Rush's and Beat's do, Tango's doesn't).
export type SupportActor = {
    kind: Kind,
    spawn: (controller: Object, eaten: Chip?) -> Object?,
    warps: boolean,
}

-- A game's Rush (@exelib/supports/rush): his kind's key and sprite, his appearance's and bite's sounds, the dust he
-- digs in and out of (effect 0x14), what his bite is, reaches and carries; the chip he leaves in the hand (EXE6's second
-- WhiCapsl; EXE5's Rush has none), and what his bite does when the opponent's navi is gone (EXE6's branch unbalances
-- the stack, EXE5's skips the hand).
export type RushLook = {
    id: string,
    sprite: SpriteAsset,
    appear_sound: Sound,
    bite_sound: Sound,
    dust: Effect,
    is: CollisionType,
    hits: CollisionType,
    status: Status,
    spared: Chip?,
    lost_target: "unbalances" | "skips",
    -- What his navi's panel must be for him to appear there, by side (nil: EXE6's `byte_80C34A8`); `pixel_nudge`
    -- false: he comes up under the opponent without EXE6's pixel down and up (EXE4's).
    stand: RushStand?,
    pixel_nudge: boolean?,
}

-- Where Rush may appear by side: the panel flags required and forbidden.
export type RushStand = { [number]: { require: number, forbid: number } }

-- A game's supports' controller (@exelib/supports/controller): its kind's key, the supports it sends, and the game's
-- warp of the host, given the controller (nil: the navi warp, `navi_chip.warp`).
export type SupportsLook = {
    id: string,
    rush: SupportActor,
    beat: SupportActor,
    tango: SupportActor,
    warp: ((controller: Object, out: boolean) -> ())?,
}

-- A patch card (BN4's, EXE5's and EXE6's Modification Cards, 改造カード;
-- docs/engine/patch-cards.md): an entry of a game's root's collection
-- `patch_cards` (the core reads it as data), what every game's card is. Its key (compat's
-- patch-cards.toml gives its number), its capacity cost (EXE6's MB, which
-- the installed cards' limit counts; none for BN4's, a card a slot) and its
-- effects in the card's order:
-- each a table with its `kind` and whether the card shows it as a `bug`,
-- with the kind's own fields. What an effect does is a game's rules' (EXE6's
-- rules/patch_cards, rules/patch_cards), and its type is the game's (EXE6's
-- `PatchCardEffect`, which its cards' constructors make). Its name is the
-- locales' (locales/<lang>.toml's `[patch_cards]`), and so is each effect's
-- line on its card screen (`[patch_card_effects]`, by the effect's kind and
-- its one choice's name). Its effects are in its card screen's order (the
-- original's effect numbers'), which the content keeps.
export type PatchCardSpec = {
    -- Its id, its key in its collection (`patch_cards.canodumb`): the
    -- loader's to read, never written in the table.
    id: string?,
    mb: number?,
    effects: { any },
}
export type PatchCard = PatchCardSpec

-- A NaviCust program (BN4's, EXE5's and EXE6's Navi Customizer parts;
-- docs/design/navicust.md): an entry of a game's root's collection
-- `navicust_programs` (the core reads it as data), what every game's program is. Its key (compat's
-- navicust.toml gives its number), the colors it comes in in its variants'
-- order, whether it is a plus part, and its shape: seven rows of seven cells
-- (`#` a cell it covers, `.` one it doesn't), centered on the middle cell, and
-- compressed (nil: it doesn't compress). What it does is a game's rules'
-- (EXE6's: `bug`, `exclusive`, `anywhere`, `effects`, read by rules/navicust). Its name is
-- the locales' (locales/<lang>.toml's `[navicust_programs]`).
export type NaviCustProgramSpec = {
    -- Its id, its key in its collection (`patch_cards.canodumb`): the
    -- loader's to read, never written in the table.
    id: string?,
    colors: { string },
    plus: boolean?,
    shape: { string },
    compressed: { string }?,
    bug: string?,
    exclusive: string?,
    anywhere: boolean?,
    effects: { NaviCustEffect }?,
}
export type NaviCustProgram = NaviCustProgramSpec

-- A game's part of the NaviCust bugs a hit inflicts (@exelib/navicust/bugs).
export type NaviCustBugTable = {
    -- Its own stats (its rules' `stats`) by the NaviStats byte a code names: the stat, and how the game reads the
    -- byte ("flag": any nonzero byte is true; "one": only 1 is; "signed": as `ldrsb` reads it).
    own: { [number]: { name: string, as: ("byte" | "flag" | "one" | "signed")? } },
    -- Bytes a code may name that no stat holds, and what they are (EXE6's version, a side's fact).
    unheld: { [number]: string }?,
    -- A drain's argument goes by its flags (EXE5's 0x0801103E): with bit 4 the level adds its low four bits (to at
    -- most 7), with bit 5 it loses them (to at least 0), else it rises to them (a level that wouldn't rise is left,
    -- the bug spared: nothing reloads). Else the argument adds (EXE6's, to at most 7).
    drain_by_flags: boolean,
    -- The codes the game's routine takes as bugs of their own.
    specials: { [number]: NaviCustBugSpecial },
    -- The side's NaviStats block, in bytes (EXE6's 0x64, EXE5's 0x60), and what a code past it does: "nothing"
    -- (EXE6's routine leaves the codes from 0x64 alone: `sub_80139F6`'s 0x08013ACC), or "written" (EXE5's writes any
    -- code's byte at 0x08011142, at NaviStats + 0x60 x side + the code, 0x08010DD6: side 0's codes 0x60 to 0xBF are
    -- side 1's bytes; the rest are RAM past both blocks, from 0x0203C940, which no stat holds and no hit of EXE5's
    -- content brings a code for, so nothing is written there; the abilities and form flags come back all the same).
    block: number,
    past_block: "nothing" | "written",
    -- The hit bug's statuses: blindness and confusion (its levels 1 and 2).
    statuses: { hit_blind: Status, hit_confuse: Status },
}

-- A code's own bug: a panel trail (its kind and level); every buster shot a blank (0xFF); MegaMan's body programs
-- gone (0xFB: EXE6's `sub_8014080`, EXE5's 0x080117D4); uninstalled, with a spark and a sound (EXE6's 0xF8,
-- `sub_80140EE`).
export type NaviCustBugSpecial =
    { kind: "panel_trail", trail: number, level: number }
    | { kind: "blanks" }
    | { kind: "strip" }
    | { kind: "uninstall", spark: Spark, sound: Sound }

-- A Beast Out lock-on mode (EXE6's rules/lockon: a record of type "lockon",
-- `new.record("lockon", { id = ..., ... })`, which only its rules read): how
-- the rush picks the panel to attack a target from. `stay`: the navi's own
-- panel. `row`: along the target's row counting from the navi's own column
-- (`same_row_offsets` when they share a row), then the rows `row_shifts`
-- away. `near`: the first of `offsets` (dx toward the user's front) next to
-- the target the navi can stand on (`far_column_offsets` for a target in the
-- column farthest ahead), then with the rules' column shifts, with a clear
-- path to the target if asked; afterwards the middle row of that column, if
-- preferred and free.
export type LockonDef = {
    id: string?,
    rule: "stay" | "row" | "near",
    offsets: { { number } }?,
    same_row_offsets: { { number } }?,
    row_shifts: { number }?,
    far_column_offsets: { { number } }?,
    column_shifts: boolean?,
    clear_path: boolean?,
    prefers_middle_row: boolean?,
}
export type Lockon = LockonDef

-- What a navi chip's navi is spawned with (@exelib/navi_chips/chip: the registers `sub_80E1880` passes to
-- `off_802CD5C[subtype]`): the controller's panel and element, and the damage word with the bonus.
export type NaviChipSpec = {
    panel_x: number,
    panel_y: number,
    element: number,
    damage: number,
}

-- An EXE6 or EXE5 navi chip's navi (@exelib/navi_chips/chip): for `user`, reporting to `controller`, as `spec` says.
export type NaviChipSummon = (user: Object, controller: Object, spec: NaviChipSpec) -> Object?

-- What an EXE6 or EXE5 navi chip's `navi` is (@exelib/navi_chips/chip): its controller's spawner (`hook`), its
-- navi's summon and its course, which EXE5's DethPhnx reads, and the rest of its spec.
export type NaviChip = {
    kind: Kind,
    hook: (user: Object, spec: DimmingChipSpec) -> Object?,
    summon: NaviChipSummon,
    course: { NaviChipStep },
    navi_block: boolean?,
    heals: boolean?,
}

-- The navi chip controllers a dimming spawner makes (@exelib/navi_chips/controller): a step of a course by phase (a
-- wait, the user warping as it starts; or the navi, waited on), EXE4's short course (the waits after the warp out and
-- after the navi), the controller's state, and what a game's `make` hands back.
-- A navi chip controller's step: a wait (the user warping out or back in as it starts, a sound then: EXE4's GutPnch's
-- warp back, 0x080E91BC), or the navi.
export type NaviChipStep = { wait: number, warp: ("out" | "in")?, sound: Sound? } | "navi"
export type NaviChipShortCourse = { out: number, after: number }
export type NaviChipControllerState = { chip: Chip?, bonus: number, busy: boolean }
export type NaviChipController = {
    kind: Kind,
    hook: (user: Object, spec: DimmingChipSpec) -> Object?,
}

-- An element flame's state (@exelib/elemrage/flame): its count left (Param1, its low seven bits), the spread to the
-- rows beside (Param1's bit 7), its time (Param2), its X offset (Param3), Param4's flags: it paralyzes (bit 2: hit
-- modifier 1, its hit back 12 ticks after one), its self type is the look's `attack_480` (bit 3: row 0x25, EXE4's
-- ElemDark's), and the panel type it leaves (the panel type the high nibble names; none, or as its state holds it
-- "none", leaves none). (Bit 0, which runs it while dimmed, no spawner sets.) Its palette (EXE5's +0x0D, EXE4's
-- +0x0E), it has hit (+0x0C) and it has sent the next ones (EXE5's +0x60).
export type ElemFlameState = {
    count: number,
    spread: boolean,
    ticks: number,
    offset: number,
    paralyzes: boolean,
    attack_480: boolean,
    leaves: PanelType?,
    palette: number,
    hit: boolean,
    sent: boolean,
}

-- A game's element flame: its key, sprite, the panel types its flames may leave (EXE4's ElemDark's poison; none, they
-- leave none), whether a spreading flame sends the two beside its next one (EXE5's; EXE4's none), its self types (an
-- attack, or row 0x25 with Param4's bit 8), what it hits, and its spark by element.
export type ElemFlameLook = {
    id: string,
    sprite: SpriteAsset,
    leaves: { PanelType }?,
    spreads: boolean,
    attack: CollisionType,
    attack_480: CollisionType,
    hits: CollisionType,
    spark: (element: number) -> Spark,
}

-- A game's element flame kind and its spawn: a flame on (x, y) with its state, the element, height and damage word,
-- of `owner`'s side and facing, of no owner.
export type ElemFlame = {
    kind: Kind,
    spawn: (owner: Object, x: number, y: number, element: number, z: number, s: ElemFlameState, damage: number) -> Object?,
}

-- A soul's navi's image as MegaMan unites with it (@exelib/souls/image: EXE5's actor object 0x2A, EXE4's effect object
-- 0x13; a soul's form's `image`): the navi's sprite, and what the image puts on as it comes in and takes off as it goes
-- (EXE5's 0x0800EDBC's and 0x0800EEBE's routines by soul: GyroMan's propeller, NapalmMan's cannon, Colonel's cape,
-- KnightMan's ball and chain, NumberMan's face; EXE4's part by 0x080E36AC's table; none: it wears nothing).
export type SoulImage = {
    sprite: SpriteAsset,
    put_on: ((image: Object) -> ())?,
    take_off: ((image: Object) -> ())?,
}

-- A game's cannonball (@exelib/cannball/ball): its kind's key, its sprite, the puff it ends in off solid ground and
-- the impact where it lands, the crack's sound, and its collision types (what it is and what it hits).
export type CannonballLook = {
    id: string,
    sprite: SpriteAsset,
    puff: Effect,
    impact: Effect,
    crack: Sound,
    is: CollisionType,
    hits: CollisionType,
}

-- A game's cannonball kind, and a thrower of it at the panel three ahead of the user.
export type Cannonball = { kind: Kind, thrower: () -> Thrower }

-- A game's Geyser (@exelib/geyser/geyser): its bomb's and water's kind keys, the bomb's and the geyser's sprites, the
-- splash (on solid ground and where the water lands) and its sound, the eruption's sound, its hits' collision types
-- (what they are, what they hit) and spark, whether the splash on solid ground takes the throw's Atk+ bonus (EXE4's
-- 0x080D5AA4 reads the spawn's +0x64; none: EXE5's, which stores it and never reads it), and its water's height: the
-- register the geyser's update leaves for the water's spawn, the battle state's address (the battle-over test's r3:
-- EXE5's 0x02034A90, EXE4's 0x02035810), at which the water's hits land and their sparks show, far above the field.
-- The Guard chips' shock wave's variant (@exelib/guard/wave, a row of the game's table): its animation, its ticks on a
-- panel, and what it does to the panels it passes.
export type GuardWaveVariant = { anim: number, ticks: number, panel: ("cracked" | "broken" | "poison")? }

-- A game's Guard shock wave (@exelib/guard/wave): its key, sheet and sound; what a plain one is, what a marking one
-- is, and what they hit; whether a segment keeps its sender (its flip and the sender as its owner); whether its first
-- tick of spreading notes the panel ahead (the phase byte).
export type GuardWaveLook = {
    id: string,
    sprite: SpriteAsset,
    sound: Sound,
    attack: CollisionType,
    thrown: CollisionType,
    hits: CollisionType,
    owned: boolean,
    notes_ahead: boolean,
}

-- A game's Guard shock wave's spawn: a segment of `owner`'s on (x, y), of a variant, with the damage word.
export type GuardWaveSpawn = (owner: Object, x: number, y: number, variant: GuardWaveVariant, damage: number) -> Object?

export type GeyserLook = {
    id: string,
    water_id: string,
    bomb: SpriteAsset,
    geyser: SpriteAsset,
    splash: Effect,
    splash_sound: Sound,
    erupt: Sound,
    attack: CollisionType,
    hits: CollisionType,
    spark: Spark,
    splash_bonus: boolean?,
    water_height: number,
}

-- A game's Geyser kinds (the bomb and its water), and a thrower of the bomb at the panel three ahead of the user.
export type GeyserBomb = { kind: Kind, water: Kind, thrower: () -> Thrower }

-- A game's NumberMan's face (@exelib/numbrman/face): its kind's key, its sprite, and EXE4's start (0x080CCD0A: with
-- the fight on it follows from its first update, its follow its action 8; none: EXE5's, which waits for every navi to
-- be in first).
export type NumberManFaceSpec = { id: string, sprite: SpriteAsset, follows_in_fight: boolean? }

-- A game's NumberMan's face kind, and its spawn on its owner.
export type NumberManFace = { kind: Kind, spawn: (owner: Object) -> Object? }

-- A die's throw (@exelib/numbrman/dice): its face (1 to 6, or EXE5's 9), its target panel, whether it is NumberMan's
-- (an attack at navis running while dimmed: its Param1), its HP, its hits' status and hit modifier, its damage word,
-- and the object waiting on it (its state's `busy`, set until it goes).
export type DieThrow = {
    face: number,
    target_x: number,
    target_y: number,
    dimmed: boolean,
    hp: number,
    status: Status?,
    hit_mod: number,
    damage: number,
    holder: Object?,
}

-- A game's die (@exelib/numbrman/dice): its kind's key and sprite; the explosion it bursts and blows up in, the puff
-- it goes in off solid ground and the one as the battle ends; EXE5's face 9's explosion (its palette 1, effect row
-- 0x24; none: EXE4's, whose faces are 1 to 6); its sounds as it blows up, bounces and bursts; its collision types
-- (NumberMan's: what it is and what it hits; the others': its body and what hits it); EXE5's markers over its target
-- (lib/navi_chips/marker; none: EXE4's, which has none); whether its burst keeps the damage's top bit apart (EXE5's
-- 0x080C51F2; none: EXE4's, the halfword's product whole); whether its init places it on its panel (EXE4's
-- 0x080CDA4A).
export type DiceSpec = {
    id: string,
    sprite: SpriteAsset,
    explosion: Effect,
    puff: Effect,
    gone: Effect,
    nine: { explosion: Effect }?,
    blow_up: Sound,
    bounce: Sound,
    burst: Sound,
    attack: CollisionType,
    hits_navis: CollisionType,
    body: CollisionType,
    hit_by: CollisionType,
    marker: ((owner: Object, x: number, y: number, ticks: number, style: number, flip: boolean) -> Object?)?,
    keeps_top_bit: boolean?,
    places_itself: boolean?,
}

-- A game's die kind, and its spawn by `owner` at `pos`.
export type Dice = { kind: Kind, spawn: (owner: Object, pos: Vec3, throw: DieThrow) -> Object? }

-- A game's NumberMan (@exelib/numbrman/navi): his kind's key, his sprite, his appearance's and throw's sounds, his face
-- and his die, and what his throw gives the die (EXE5's 0x080BB4C2 word 0x30014: 20 HP, no status, hit modifier 3;
-- EXE4's die states its own, 0x080CDA44: 40 HP, hit modifier 3).
export type NumberManSpec = {
    id: string,
    sprite: SpriteAsset,
    appear: Sound,
    throw: Sound,
    face: NumberManFace,
    dice: Dice,
    die: { hp: number, status: Status?, hit_mod: number },
}

-- A game's NumberMan kind, and what makes one just spawned his chip's: the controller he reports to, his palette.
export type NumberMan = { kind: Kind, summoned: (o: Object, controller: Object, palette: number) -> () }

-- A Geddon chip's quake's change (@exelib/geddon/quake's `change`): what it does to a panel, whether it changed, and
-- what shows over a panel that did.
export type GeddonChange = { apply: (x: number, y: number) -> boolean, show: (at: Vec3) -> () }

-- What ShadeMan's crush carries (@exelib/shademan): his figure's palette, its hits' hit modifier and status, and a
-- NaviCust bug's code and argument (none: 0).
export type ShadeManHit = { palette: number, hit_mod: number, status: Status?, bug: number?, bug_arg: number? }

-- A game's ShadeMan's crush (@exelib/shademan/crush): its kind's key, its sprite and sound, its hits' collision types
-- (what they are and what they hit), where its figure stands from its panel's center (`place`), the ticks left when
-- its second hit comes, and whether it highlights the panels it reaches each tick (EXE5's).
export type ShadeManCrushSpec = {
    id: string,
    sprite: SpriteAsset,
    sound: Sound,
    attack: CollisionType,
    hits: CollisionType,
    place: (me: Object) -> (),
    second_at: number,
    highlights: boolean,
}

-- A game's ShadeMan's crush kind, and its spawn by ShadeMan on (x, y) carrying `given`.
export type ShadeManCrush = { kind: Kind, spawn: (shademan: Object, x: number, y: number, given: ShadeManHit) -> Object? }

-- A game's ShadeMan (@exelib/shademan/navi): his kind's key, his sprite, his animations as he appears and as he calls
-- out, his sounds (and EXE5's as a command is in), his crush, his commands (each its steps, a step any of its keys,
-- and what it gives the crush) in the order they are read, what the crush carries without one (drawn as he appears),
-- whether he bows before he goes (EXE5's), and EXE4's puff as he goes.
export type ShadeManSpec = {
    id: string,
    sprite: SpriteAsset,
    appear_anim: number,
    call_anim: number,
    appear: Sound,
    call: Sound,
    command_sound: Sound?,
    crush: ShadeManCrush,
    commands: { { steps: { { Key } }, hit: ShadeManHit } },
    uncommanded: () -> ShadeManHit,
    bows: boolean,
    puff: Effect?,
}

-- A game's ShadeMan kind, and what makes one just spawned his chip's: the controller he reports to, his palette.
export type ShadeMan = { kind: Kind, summoned: (o: Object, controller: Object, palette: number) -> () }

-- What BugFix's glow (@exelib/bugfix/glow) does first, its spawner's (`glow.start`): its first action, which sets
-- action 4 when it is done.
export type BugFixGlowStart = { run: (me: Object) -> () }
