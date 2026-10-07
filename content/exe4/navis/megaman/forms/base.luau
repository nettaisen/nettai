--!strict
-- EXE4's MegaMan's base form (`base`, the engine's base form of EXE4's game: rules-in-luau.md P1c): MegaMan in no soul
-- (NaviStats +0x24 0), his sprite with the soul's number 0 added (0x0800B90A). His weapons in it are his navi's.
--
-- Not yet (docs/design/exe4-map.md §18): his faces in the emotion window, and the souls.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged_shot")

local form: FormDef = {
    base = true,
    sprite = asset.sprite("megaman"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
}

return { base = form }
