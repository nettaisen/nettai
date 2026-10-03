-- Types the shared modules (content/common) and the games' modules that use them both name, declared once (a
-- definitions file like each game's types.d.luau; the checker reads every folder's).

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
-- of the 3x3), the sounds of its landing, its crushing and its burst; and the bug its burst gives (setting the
-- collision's bugs from its target's stats).
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
}

-- A game's snake (@common/snake/snake): its sprite and sounds, the ticks it waits before its leap, its strike's
-- element, spark, hit modifier and collision types, and how it tells its nest it's out (`joined`) and gone (`left`).
export type SnakeLook = {
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

-- A chip's tank cannon (@common/tankcan/action): the action's key, the cannon it holds, the shell kind and how its
-- shell flies (`ShellSpec`: BN6's types).
export type TankCanSpec = { id: string?, cannon: AttachmentLook, kind: Kind, shell: ShellSpec }

-- A yoyo throw (@common/yoyo/throw): the action's key, the yoyo arm, and what it throws (from (x, y), `z` up, with
-- the damage word).
export type YoyoThrowSpec = {
    id: string,
    arm: AttachmentLook,
    throw: (me: Object, x: number, y: number, z: number, damage: number) -> Object?,
}
