--!strict
-- AntiDmg's shuriken (the original's attack object #0xC2, `sub_80DD764`):
-- the shared shuriken (@exelib/antidmg/shuriken), BN6's kind and look: a
-- thrown slash (row 0x1C) that hits navis.

local collision = require("../../rules/collision")
local common = require("@exelib/antidmg/shuriken")
local effects = require("../../lib/effects")

local shuriken = {}

shuriken.kind = common.make_kind {
    id = "antidmg/shuriken",
    sprite = asset.sprite("shuriken"),
    land_sound = asset.sound("panel-crack"),
    puff = effects.bubbles,
    self_type = collision.thrown_slash,
    target = collision.hits_navis,
}

return shuriken
