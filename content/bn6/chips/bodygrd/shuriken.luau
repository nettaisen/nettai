--!strict
-- BodyGrd's shuriken (the original's attack object #0x5C, `sub_80CFEC4`):
-- the shared shuriken (@common/bodygrd/shuriken) with BN6's look: it dives
-- from 180 pixels up and behind onto its panel in 16 ticks, highlighting
-- it, appearing with a puff halfway, hits it for two ticks, lies there 30
-- ticks and blinks out over 20.

local collision = require("../../rules/collision")
local common = require("@common/bodygrd/shuriken")
local effects = require("../../lib/effects")

local shuriken = {}

shuriken.kind = common.make_kind {
    id = "bn6:bodygrd/shuriken",
    sprite = asset.sprite("bn6:shuriken"),
    land_sound = asset.sound("bn6:panel-crack"),
    puff = effects.bubbles,
    -- What it is (row 0x1C: a thrown slash) and hits.
    is = collision.thrown_slash,
    hits = collision.hits_navis,
}

-- `sub_80D006E`: a shuriken onto (x, y) from `owner` with `element`,
-- `hit_mod` and the damage word.
function shuriken.spawn(owner: Object, x: number, y: number, element: number, hit_mod: number, damage: number): Object?
    return common.spawn(shuriken.kind, owner, x, y, element, hit_mod, damage)
end

return shuriken
