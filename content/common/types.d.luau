-- The shared types of content/common's modules (docs/design/rules-in-luau.md): what a game passes them (its looks,
-- kinds and constants) and what they hand back. Every content folder's definitions check together, so BN5's and
-- BN6's modules use these as their own.

-- How an attachment (@common/attachment/attachment) animates: its animation, whether it animates while dimmed too,
-- and what is added to its palette (drawn only).
export type AttachmentOptions = { anim: number?, while_dimmed: boolean?, palette_add: number? }

-- A game's sonic boom (@common/swords/sonic_boom): its sprite, its height, its palette when it goes through, else by
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

-- A game's buster parts (@common/buster): the muzzle flash a shot leaves, the buster arm, an enemy navi's arm, the
-- recovery after a shot (by Rapid, then the open panels ahead), and its attachment kind.
export type BusterLook = {
    flash: AttachmentLook,
    arm: AttachmentLook,
    navi_arm: AttachmentLook,
    recovery: { { number } },
    attachment: Kind,
}

-- What the buster's shots share, made of a game's `BusterLook` (@common/buster).
export type Buster = {
    flash: AttachmentLook,
    arm: AttachmentLook,
    raise_arm: (navi: Object) -> (),
    raise_arm_for: (navi: Object, holder: Object, which: Slot, while_dimmed: boolean) -> (),
    recovery: (navi: Object, x: number, y: number) -> number,
    move_cancel: (navi: Object) -> boolean,
}

-- Firing a game's projectile (@common/projectile): from any panel, or from the panel in front as the buster does (the
-- buster's recovery from there).
export type Projectiles = {
    spawn: (owner: Object, x: number, y: number, s: ProjectileShot) -> Object?,
    fire: (navi: Object, s: ProjectileShot) -> number,
}

-- A game's shooting chips (@common/cannon, @common/airshot): the shot's sound, the attachment kind (what the navi
-- holds) and its projectiles.
export type ShooterLook = { sound: Sound, attachment: Kind, projectiles: Projectiles }

-- A game's throw (@common/bombs/throw): the wind-up's sound, and its attachment kind (what the navi holds).
export type ThrowLook = { sound: Sound, attachment: Kind }

-- A game's thrown bomb (@common/bombs/bomb): its kind's key, its sprite, its landing's sound, the splash in a hole,
-- the explosion over its blast and the puff when the battle ends.
export type BombLook = { id: string, sprite: SpriteAsset, hit_sound: Sound, splash: Effect, explosion: Effect, puff: Effect }

-- A game's thrown bombs, made of its `BombLook` (@common/bombs/bomb): the kind, and a thrower of a variant.
export type Bombs = { kind: Kind, thrower: (variant: BombVariant) -> Thrower }

-- Aiming thrown things over one game's sine table (@common/trajectory).
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

-- A game's wide wave (@common/widesht/wave): its kind's key, its sprite and sound, its afterimages' sprite (a
-- variant's own overrides both), and what it hits.
export type WideWaveLook = { id: string, sprite: SpriteAsset, sound: Sound, trail_sprite: SpriteAsset, target: CollisionType }

-- A game's wide shots (@common/widesht/action): its wide shooter (a spec's own overrides it), its attachment kind and
-- its wave's kind.
export type WideShotLook = { shooter: AttachmentLook, attachment: Kind, wave: Kind }

-- A wide shot: its waves (`shooter` and `z`, where a chip's aren't its game's wide shooter and 16 pixels).
export type WideShotSpec = { id: string?, wave: WaveVariant, waves: number, shooter: AttachmentLook?, z: number? }

-- What a chip's vulcan fires (@common/vulcan/action): how many bullets, and which; the vulcan the navi holds and the
-- bullets' kind (none: the game's).
export type VulcanSpec = { shots: number, bullet: BulletVariant, arm: AttachmentLook?, bullet_kind: Kind? }

-- A game's vulcans (@common/vulcan/action): the shot's sound, the vulcan the navi holds, its attachment kind and its
-- bullets' kind.
export type VulcanLook = { sound: Sound, arm: AttachmentLook, attachment: Kind, bullet_kind: Kind }

-- What a chip's blow raises (@common/tornado/blow): one tornado (`single`), or one on each panel of `spread` at a
-- level, by the navi's kinds of NaviCust bug or by its side's statistic 3 (`by`; `tornadoes`, by level, over
-- `tornado`: tornadoes that differ by level, BN5's Static's); and the fan it holds, with an addition to its palette.
export type TornadoBlow = {
    -- The action's key when no chip definition holds it.
    id: string?,
    fan: AttachmentLook,
    fan_palette: number?,
    -- The tornadoes' kind (none: the game's).
    tornado_kind: Kind?,
    single: TornadoSpec?,
    spread: {
        by: "bug_kinds" | "side_stat",
        panels: { [number]: { { number } } },
        tornado: TornadoSpec,
        tornadoes: { [number]: TornadoSpec }?,
    }?,
}

-- A game's blows (@common/tornado/blow): its attachment kind (the fan's) and its tornadoes' kind.
export type TornadoBlowLook = { attachment: Kind, tornado: Kind }

-- A chip's thunder shot (@common/thunder/shoot): its ball; `id`, the action's key when no chip definition holds it;
-- `kind`, a ball kind of its own (none: the game's).
export type ThunderShot = { id: string?, ball: ThunderBall, kind: Kind? }

-- A game's thunder shots (@common/thunder/shoot): its buster (the arm) and its ball's kind.
export type ThunderShotLook = { buster: Buster, ball: Kind }

-- How a falling meteor's panel marker blinks (@common/meteors/marker; BN6's `byte_80E4334`, by the original's first
-- parameter): its lifetime, its blink period, the ticks lit each period, and how much each period shrinks the next
-- (doubling).
export type MeteorMarkerLook = {
    lifetime: number,
    period: number,
    lit: number,
    shrink: number,
}

-- A game's boomerang (@common/boomer/boomerang): its kind's key, its sprites (the tomahawk's too) and sound, what it
-- is (where a variant doesn't say) and hits, and its spark (where a variant doesn't say).
export type BoomerangLook = {
    id: string,
    sprite: SpriteAsset,
    tomahawk_sprite: SpriteAsset,
    sound: Sound,
    is: CollisionType,
    hits: CollisionType,
    spark: Spark,
}

-- A boomerang's variant: 16.16 pixels a tick along a row and along the
-- column, whether it turns the other side's panels it crosses to grass,
-- and whether it is the tomahawk (its own sprite and shadow, and its
-- second animation off row 1); its sprite, collision type and spark where
-- a game's kind's aren't its (BN5's Boomer).
export type BoomerangVariant = {
    speed: number,
    turn_speed: number,
    grass: boolean,
    tomahawk: boolean?,
    sprite: SpriteAsset?,
    is: CollisionType?,
    spark: Spark?,
}

-- How a boomerang is thrown: its variant; the column (0 to 2) where it
-- turns, or `turns = false` (the original's Param2 3: it flies straight
-- along its row); a stronger hit (hit modifier 3, else 1).
export type BoomerangThrow = { variant: BoomerangVariant, turns: boolean?, column: number?, strong: boolean? }

-- A game's ProtoMan (@common/protoman/navi): his kind's key, the sounds of his coming and his swing, his attachment
-- kind (his sword's), and his slash's collision types (what it hits, and what it is).
export type ProtoManKind = {
    id: string,
    appear_sound: Sound,
    swing_sound: Sound,
    attachment: Kind,
    target: CollisionType,
    self_type: CollisionType,
}

-- A game's MegaMan's shots (@common/megaman): the action's key, its buster parts, its projectiles, its attachment
-- kind, the shot's sound, and the projectile a shot fires without a setup's (the table's first row).
export type MegaManShotLook = {
    id: string,
    buster: Buster,
    projectiles: Projectiles,
    attachment: Kind,
    shot_sound: Sound,
    plain: ProjectileVariant,
}

-- A game's buster shot (@common/megaman/buster): its shot's look, and a throw's sound and its throw of the obstacle
-- the navi absorbed (none: the game throws none).
export type MegaManBusterLook = MegaManShotLook & { throw_sound: Sound, throw: ((me: Object) -> ())? }

-- What a spreader fires (@common/spreadr/action): its gun (BN6's attachment rows 0x15 and 0x2F, by `dword_80ECC54`),
-- the bullet's height (16.16, by `dword_80ECC5C`), and the bullet; its own bullet kind and shot sound (none: the
-- game's; BN5's Spreader's).
export type SpreaderSpec = {
    id: string?,
    gun: AttachmentLook,
    z: number,
    bullet: BulletVariant,
    bullet_kind: Kind?,
    sound: Sound?,
}

-- A game's spreaders (@common/spreadr/action): the shot's sound, the attachment kind (the gun's) and its bullets'
-- kind.
export type SpreaderLook = { sound: Sound, attachment: Kind, bullet_kind: Kind }

-- A trap chip's object (a "trap" record, @common/traps/controller): `set` spawns it for the controller and its user,
-- with the chip and bonus the controller keeps.
export type Trap = {
    set: (controller: Object, user: Object, chip: Chip?, bonus: number) -> Object?,
}

-- A hockey puck's variant (@common/airhocky/puck: a row of BN6's `byte_80C9818`, BN5's 0x080CA3A4): the steps it
-- lasts (0: no limit), how it bounces (1: off its own side too, once across), the ticks per step, and whether it
-- leaves a trail.
export type PuckVariant = { steps: number, own_side: number, ticks: number, trail: boolean? }

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
    trail_tint: number,
}

-- A hockey chip's flick (@common/airhocky/flick): the action's key, its puck's kind and variant, and whether it
-- goes down the field.
export type FlickSpec = { id: string, kind: Kind, puck: PuckVariant, down: boolean }

-- A sun beam's look (@common/gundels/beam): its sprite and palette.
export type SunBeamLook = { sprite: SpriteAsset, palette: number }

-- A game's BugBomb (@common/bugbomb/bomb): its sprite and animation, what it is and is hit by while it sits (and
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

-- A game's snake (@common/snake/snake): its sprite and sounds, the ticks it waits before its leap, its strike's
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

-- A snake's state: the panel it leaps at (x 0xFF: none), its nest, and its place in its nest's flags (BN5's).
export type SnakeState = { target_x: number, target_y: number, nest: Object?, slot: number }

-- A nest's scan: where it is (bytes, the column stepping off the field when the scan ends).
export type SnakeScan = { scan_x: number, scan_y: number }

-- A game's tank shell (@common/tankcan/shell): what it is and hits, its sounds (the shot, a hit or blast), the
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

-- A chip's tank cannon (@common/tankcan/action): the action's key, the cannon it holds and its game's attachment
-- kind, the shell kind and how its shell flies (`ShellSpec`: BN6's types).
export type TankCanSpec = { id: string?, cannon: AttachmentLook, attachment: Kind, kind: Kind, shell: ShellSpec }

-- A yoyo throw (@common/yoyo/throw): the action's key, the yoyo arm and its game's attachment kind, and what it
-- throws (from (x, y), `z` up, with the damage word).
export type YoyoThrowSpec = {
    id: string,
    arm: AttachmentLook,
    attachment: Kind,
    throw: (me: Object, x: number, y: number, z: number, damage: number) -> Object?,
}

-- A game's moon blade (@common/moonbld/blade): its sprite, and what it is and hits.
export type MoonBladeLook = { sprite: SpriteAsset, is: CollisionType, hits: CollisionType }

-- A game's thrown seed (@common/bombs/seed): its sprite and animation, what it is and hits and its hit modifier, the
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

-- A game's H-Burst shot (@common/h-burst/burst): its probe's self type, what its bursts are and reach, their status
-- and the explosion on each of their panels.
export type HyperBurstShotLook = {
    probe: CollisionType,
    is: CollisionType,
    hits: CollisionType,
    status: Status,
    explosion: Effect,
}

-- A game's H-Burst (@common/h-burst/action): the spreader the navi holds, its attachment kind, the shot's sound and
-- the shot's kind.
export type HyperBurstLook = { spreader: AttachmentLook, attachment: Kind, sound: Sound, burst: Kind }

-- An H-Burst (BN6's action 0x34, BN5's 0x35): the bursts its shot makes where it hits.
export type HyperBurstSpec = { id: string?, bursts: number }

-- A game's BodyGrd shuriken (@common/bodygrd/shuriken): its kind's key, its sprite, the landing's sound, the puff it
-- appears in, what it is and hits.
export type BodyGuardShurikenLook = {
    id: string,
    sprite: SpriteAsset,
    land_sound: Sound,
    puff: Effect,
    is: CollisionType,
    hits: CollisionType,
}

-- A game's BodyGrd striker (@common/bodygrd/striker): its kind's key, its identity, the shurikens' kind and the
-- throw's sound.
export type BodyGuardStrikerLook = { id: string, identity: Identity, shuriken: Kind, throw_sound: Sound }
