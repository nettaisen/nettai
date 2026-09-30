-- Types shared across the bn6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/bn6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API and data types are in core.d.luau.)

-- Where an owner keeps an attached object: its actor overlay slot, or its
-- first related object (lib/slot.luau).
export type Slot = "overlay" | "related"

-- The state of VarSwrd's and NeoVari's actions (lib/vari_sword.luau).
export type VariSwordState = {
    -- Ticks left to enter a sequence (AIAttackVars+0x12).
    countdown: number,
    -- The sequences as the navi faces: its direction flip.
    flip: number,
    -- How far into each sequence the navi is, in bytes (two per step).
    progress: { number },
}

-- What differs between VarSwrd and NeoVari (lib/vari_sword.luau).
export type VariSwordSpec = {
    -- The sequences by direction flip (0, 1), each a list of joypad words
    -- ending in 0.
    sequences: { [number]: { { number } } },
    -- The chip each sequence gives.
    choices: { number },
    -- A random pick is `choices` by `rng % random`, a Sword past them.
    random: number,
    -- The chip that doesn't turn into the charged sword (action 0x41).
    no_charged: number,
}
