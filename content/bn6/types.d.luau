-- Types shared across the bn6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/bn6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack.

-- Where an owner keeps an attached object: its actor overlay slot, or its
-- first related object (lib/slot.luau).
export type Slot = "overlay" | "related"

export type SpriteId = { category: number, index: number }

-- What an attachment looks like and where it sits on its owner
-- (data/attacks.luau ATTACHMENTS).
export type AttachmentKind = {
    sprite: SpriteId,
    palette: number,
    -- Pixels raised by (subtracted from y and z).
    lift: number,
    -- The owner's sprite attach point it follows (nil: its origin).
    attach_point: number?,
}

-- Which sun beam sprite and palette GunDelSol shows
-- (data/attacks.luau GUN_DEL_SOL_BEAMS).
export type SunBeamLook = { sprite: number, palette: number }
