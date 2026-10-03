--!strict
-- BN5's MegaMan's base form (`bn5:base`, the engine's base form of BN5's game: rules-in-luau.md P1c): MegaMan in no
-- soul (NaviStats +0x2C 0), his sprite by his soul (0x0800DA58[0]). His weapons in it are his navi's, from his stats.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged-shot")

return define.form {
    id = "bn5:base",
    kind = "base",
    sprite = asset.sprite("bn5:megaman"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    buster_arm = { anim = 0 },
}
