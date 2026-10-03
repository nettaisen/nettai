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

-- A trap chip's object (a "trap" record, @common/traps/controller): `set` spawns it for the controller and its user,
-- with the chip and bonus the controller keeps.
export type Trap = {
    set: (controller: Object, user: Object, chip: Chip?, bonus: number) -> Object?,
}
