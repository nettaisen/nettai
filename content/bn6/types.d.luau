-- Types shared across the bn6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/bn6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API and data types are in core.d.luau.)

-- Where an owner keeps an attached object: its actor overlay slot, its
-- first related object, or its content state's `held` field
-- (lib/slot.luau).
export type Slot = "overlay" | "related" | "held"

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
    -- A speed, 16.16 (AIAttackVars+0x34), or a second object there.
    speed: number,
    held2: Object?,
}

-- What a projectile (attack object #0, objects/projectile) is fired with
-- (lib/projectile.luau).
export type ProjectileShot = {
    -- The kind (data.objects.projectiles): its first parameter.
    kind: number,
    -- The damage word: the damage (with its flag bits) | the counter
    -- byte << 16.
    damage: number,
    -- Its height, 16.16 (the buster's shots fly at 24 pixels, 0x18_0000).
    z: number,
    -- Rows it also moves on its first step: 1 down, -1 up (its second
    -- parameter). None by default.
    row: number?,
}

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
