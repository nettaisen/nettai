--!strict
-- MegaMan's base form.

local buster = require("../../weapons/buster/weapon")
local charged_shot = require("../../weapons/charged-shot/weapon")

return define.form {
    id = "base",
    name = "Base",
    kind = "base",
    sprite = asset.sprite("megaman"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    -- The buster arm it raises (lib/buster).
    buster_arm = { anim = 0 },
}
