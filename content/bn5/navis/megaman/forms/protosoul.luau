--!strict
-- ProtoSoul (soul 1, Team ProtoMan's): BN5's Soul Unison with ProtoMan, for a sword chip given up. MegaMan's sprite
-- in it is 00-01 (0x0800DA58[1]); he changes by BN5's soul change (rules/souls).

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged-shot")
local souls = require("../../../rules/souls/system")

return define.form {
    id = "bn5:protosoul",
    kind = "soul",
    soul = { number = 1, family = "sword" },
    change = souls.action,
    sprite = asset.sprite("bn5:protosoul"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    buster_arm = { anim = 0 },
}
