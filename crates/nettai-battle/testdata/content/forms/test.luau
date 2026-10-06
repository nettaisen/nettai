--!strict
-- The engine's test navis' one form, MegaMan's base form (../navis/test),
-- with the EXE6 overlay's buster and charged shot.

local buster = require("../navis/megaman/weapons/buster")
local charged_shot = require("../navis/megaman/weapons/charged_shot")

local forms: { [string]: FormDef } = {
    base = {
        base = true,
        sprite = asset.sprite("test-navi"),
        element = "null",
        buster_bonus = 0,
        weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
        buster_arm = { anim = 0 },
    },
}

return forms
