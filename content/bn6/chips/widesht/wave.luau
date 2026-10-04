--!strict
-- The wide wave (the original's attack object #0x3B, `sub_80CB49C`): the
-- shared wave (@exelib/widesht/wave), BN6's kind, look and variants.
-- WideSht's ends on its first hit; SuprSpr's goes through, hitting each
-- panel once. See docs/engine/shot-chips.md §10.1.

local collision = require("../../rules/collision")
local common = require("@exelib/widesht/wave")

local wave = {}

-- A wave variant.
wave.variant = common.variant

wave.kind = common.make_kind {
    id = "bn6:widesht/wave",
    sprite = asset.sprite("bn6:wide-wave"),
    sound = asset.sound("bn6:wide-shot"),
    trail_sprite = asset.sprite("bn6:rock-cubes"),
    target = collision.hits_navis,
}

-- `sub_80CB686`: a wave of `variant` from `owner` on panel (x, y) at
-- height `z` (16.16), with `element`, the damage word and the hit
-- modifier a piercing variant uses; nil if the attack pool is full.
function wave.fire(owner: Object, x: number, y: number, z: number, variant: WaveVariant, element: number, damage: number, hit_mod: number): Object?
    return common.fire(wave.kind, owner, x, y, z, variant, element, damage, hit_mod)
end

-- The variants (the original's kinds 0 to 9: its speed, 16.16 pixels a
-- tick, by `byte_80CB58C`). Kinds 0 to 2 hold their spawner and wait
-- before sliding, and kind 9 trails afterimages and bugs what it hits;
-- nothing in the game fires those.
local function v(speed: number, more: { pierces: boolean?, holds: boolean?, trails: boolean? }?): WaveVariant
    local m: { pierces: boolean?, holds: boolean?, trails: boolean? } = more or {}
    return wave.variant {
        speed = speed,
        pierces = m.pierces or false,
        holds = m.holds or false,
        trails = m.trails or false,
        collision = if m.trails then collision.attack_480 else collision.attack,
        -- `sub_801A4D0(0x14, 2)` and palette 1.
        bug = if m.trails then 0x214 else nil,
        palette = if m.trails then 1 else nil,
    }
end
local ROWS: { [number]: WaveVariant } = {
    [0] = v(0x3_0000, { holds = true }),
    [1] = v(0x3_8000, { holds = true }),
    [2] = v(0x4_0000, { holds = true }),
    [3] = v(0x6_0000),
    [4] = v(0x6_8000),
    [5] = v(0x7_0000),
    [6] = v(0x6_0000, { pierces = true }),
    [7] = v(0x6_8000, { pierces = true }),
    [8] = v(0x7_0000, { pierces = true }),
    [9] = v(0x7_0000, { pierces = true, trails = true }),
}
wave.variants = {
    -- WideSht's (kind 3).
    wide = ROWS[3],
    -- SuprSpr's (kind 6): it goes through.
    super = ROWS[6],
}

-- The variant of kind `n`, for the callers that name one by number; past
-- the table the game reads on into the next routine's code: an error.
function wave.by_number(n: number): WaveVariant
    local r = ROWS[n]
    if not r then
        error(string.format("wave kind %#x reads past byte_80CB58C", n))
    end
    return r
end

return wave
