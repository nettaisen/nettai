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

-- A trap chip's object (a "trap" record, @common/traps/controller): `set` spawns it for the controller and its user,
-- with the chip and bonus the controller keeps.
export type Trap = {
    set: (controller: Object, user: Object, chip: Chip?, bonus: number) -> Object?,
}
