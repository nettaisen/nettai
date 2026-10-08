--!strict
-- NumberSoul (soul 8): EXE4's Double Soul with NumberMan, for a plus chip given up (0x08020008[7]).
-- MegaMan's sprite in it is 00-08, his face picture 12 (0x08014B78), NumberMan's image numberman (08-08). (What the
-- souls share: forms/soul.)

local chip = require("@self/chip").numsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "plus", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("numberman")),
    sprite = asset.sprite("numbersoul"),
    element = "null",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("numbersoul") },
}

return { numbersoul = form }
