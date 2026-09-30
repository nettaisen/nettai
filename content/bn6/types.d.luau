-- Types shared across the bn6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/bn6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack.

-- Where an owner keeps an attached object: its actor overlay slot, or its
-- first related object (lib/slot.luau).
export type Slot = "overlay" | "related"

export type SpriteId = { category: number, index: number }

-- What an attachment looks like and where it sits on its owner
-- (data/pack.luau `attachments`).
export type AttachmentKind = {
    -- The attachment's number (its first spawn parameter).
    id: number,
    sprite: SpriteId,
    palette: number,
    -- Pixels raised by (subtracted from y and z).
    lift: number,
    -- The owner's sprite attach point it follows (nil: its origin).
    attach_point: number?,
}

-- Which sun beam look (data/pack.luau `sun_beam_looks`) and palette
-- GunDelSol shows.
export type SunBeamLook = { look: number, palette: number }

-- GunDelSol's data, in its chip's folder of the content pack.
export type GunDelSol = {
    -- Ticks of hits under the beam.
    firing_ticks: number,
    -- The sun beam in the shade and in the sun.
    beam: SunBeamLook,
    beam_in_sun: SunBeamLook,
    -- The gun attached to the user.
    gun: AttachmentKind,
}

-- A chip's own data (data/pack.luau `chips`).
export type ChipData = {
    gun_del_sol: GunDelSol?,
}
