-- Types shared across the bn6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/bn6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API and data types are in core.d.luau.)

-- Where an owner keeps an attached object: its actor overlay slot, or its
-- first related object (lib/slot.luau).
export type Slot = "overlay" | "related"

-- What the link navis' chips (action 0x0A, chips/190-heatpres) keep in the
-- attack's variables: one schema for the ten navis' routines.
export type LinkChipState = {
    -- The phase timer (AIAttackVars+0x10).
    timer: number,
    -- A destination panel (AIAttackVars+0x16/0x17).
    target_x: number,
    target_y: number,
    -- The object the routine keeps (AIAttackVars+0x30).
    held: Object?,
    -- A speed, 16.16 (AIAttackVars+0x34).
    speed: number,
}
