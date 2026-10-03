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

-- A trap chip's object (a "trap" record, @common/traps/controller): `set` spawns it for the controller and its user,
-- with the chip and bonus the controller keeps.
export type Trap = {
    set: (controller: Object, user: Object, chip: Chip?, bonus: number) -> Object?,
}
