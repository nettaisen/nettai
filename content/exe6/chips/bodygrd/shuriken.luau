--!strict
-- BodyGrd's shuriken (the original's attack object #0x5C, `sub_80CFEC4`):
-- the shared shuriken (@exelib/bodygrd/shuriken), EXE6's kind and look: a
-- thrown slash (row 0x1C) that hits navis. It dives from 180 pixels up and
-- behind onto its panel in 16 ticks, highlighting it, appearing with a
-- puff halfway, hits it for two ticks, lies there 30 ticks and blinks out
-- over 20.

local collision = require("../../rules/collision")
local common = require("@exelib/bodygrd/shuriken")
local effects = require("../../lib/effects")

local shuriken = {}

shuriken.kind = common.make_kind {
    id = "bodygrd/shuriken",
    sprite = asset.sprite("shuriken"),
    land_sound = asset.sound("panel-crack"),
    puff = effects.bubbles,
    self_type = collision.thrown_slash,
    target = collision.hits_navis,
}

-- `sub_80D006E`: a shuriken onto (x, y) from `owner` with `element`,
-- `hit_mod` and the damage word, of its side and facing.
function shuriken.spawn(owner: Object, x: number, y: number, element: number, hit_mod: number, damage: number): Object?
    return common.spawn(shuriken.kind, owner, x, y, element, hit_mod, damage)
end

return shuriken
