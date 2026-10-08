-- Types shared across the exe4 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `.zed/generate-luau-definitions.py`; see README.md): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API is in content/nettai/core.d.luau.)

-- What a patch card's effect works on in the reload besides the stats (rules/patch_cards): the HP the NaviCust's
-- programs and the cards add.
export type PatchCardWork = { hp: number }

-- A ring (objects/ring, attack object 0x6D: one of the 3-byte rows at 0x080DAFB0, which each thrower states): its
-- sheet, its speed in pixels a tick, its hits' status (none: none), its sound as it sets off, its hit modifier, and
-- the panel type its hit leaves (SandRing's sand; none: none).
export type RingVariant = {
    sprite: SpriteAsset,
    speed: number,
    status: Status?,
    sound: Sound,
    hit_mod: number,
    sand: PanelType?,
}

-- A breath chip's breath (chips/breath, attack object 0x36): its own panel was hit (+0x60), and the column's three
-- ahead (+0x64, +0x68, +0x6C: level, up, down); its sound's count (+0x22).
export type BreathState = {
    hit: boolean,
    ahead: boolean,
    above: boolean,
    below: boolean,
    count: number,
}

-- A flying shot's variant (objects/flying_shot: one of the 16-byte rows at 0x080CE5B0, which each user defines where
-- it fires it).
export type FlyingShotVariant = {
    -- Its collision (the row's first three bytes: what it is, what it hits, the hit modifier).
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    -- Its element byte.
    element: number,
    -- Its hit spark (0xFF: none).
    spark: Spark?,
    sprite: SpriteAsset,
    anim: number,
    -- It casts a shadow on the ground (else its first part is drawn with the sprite).
    shadow: boolean,
    -- It lights the panels it covers (its second parameter).
    highlight: boolean,
    -- The panel centers it passes before it ends (its third parameter).
    range: number,
    -- 16.16 a tick, forward.
    speed: number,
    -- What shows at the panel's center as its range runs out (row 5's: effect row 0x56).
    end_effect: Effect?,
}

-- The NaviCust's bug counts (rules/navicust's), which a card raises or clears.
export type PatchCardCounts = {
    add: (bug: string, n: number) -> (),
    clear: (bug: string) -> (),
}
