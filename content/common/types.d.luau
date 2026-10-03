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

-- A trap chip's object (a "trap" record, @common/traps/controller): `set` spawns it for the controller and its user,
-- with the chip and bonus the controller keeps.
export type Trap = {
    set: (controller: Object, user: Object, chip: Chip?, bonus: number) -> Object?,
}
