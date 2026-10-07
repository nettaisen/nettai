--!strict
-- EXE4's MegaMan's base form (`base`, the engine's base form of EXE4's game: rules-in-luau.md P1c): MegaMan in no soul
-- (NaviStats +0x24 0), his sprite with the soul's number 0 added (0x0800B90A). His weapons in it are his navi's.
--
-- His faces in the emotion window (0x08014B78: pictures 0 to 4, each bringing its box), by the emotions the rules
-- give (EXE5's emotions until EXE4's are ported, docs/design/exe4-map.md §7, §18 item 8: the faces are in EXE5's order).
--
-- Not yet (docs/design/exe4-map.md §18): the souls.

local buster = require("../weapons/buster")
local charged_shot = require("../weapons/charged_shot")

local form: FormDef = {
    base = true,
    sprite = asset.sprite("megaman"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    palette = 0,
    mugshot = {
        normal = asset.mugshot("megaman"),
        angry = asset.mugshot("megaman-angry"),
        worried = asset.mugshot("megaman-worried"),
        full_synchro = asset.mugshot("megaman-full-synchro"),
        worn_out = asset.mugshot("megaman-dark"),
    },
}

return { base = form }
