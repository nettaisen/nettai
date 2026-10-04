--!strict
-- BN5's AntiDmg's counter (navi action 0x47) and shuriken (attack object #0xC2): BN6's code (@exelib/antidmg), BN5's
-- definitions; the shuriken's look and collision types, the sounds and the puff BN6's. The ruleset starts it by role
-- (`actions.anti_damage_counter`, rules/roles.luau): a trap's catch leaves the attack's variant 0, a random enemy;
-- ShadowSoul's stance (navis/megaman/forms/shadowsoul/back) 1, the nearest enemy ahead.

local collision = require("../../rules/collision")
local common = require("@exelib/antidmg/counter")
local effects = require("../../lib/effects")
local shuriken = require("@exelib/antidmg/shuriken")

local counter = {}

local shuriken_kind = shuriken.make_kind {
    id = "bn5:antidmg/shuriken",
    sprite = asset.sprite("bn5:shuriken"),
    land_sound = asset.sound("bn5:panel-crack"),
    puff = effects.bubbles,
    self_type = collision.thrown_slash,
    target = collision.hits_navis,
}

counter.action = common.action_at {
    id = "bn5:antidmg/counter",
    shuriken = shuriken_kind,
    vanish_sound = asset.sound("bn5:trap-vanish"),
    throw_sound = asset.sound("bn5:boomerang"),
    puff = effects.bubbles,
}

return counter
