--!strict
-- RollSoul (soul 1): EXE4's Double Soul with Roll, for a recovery chip given up (0x08020008[0]). MegaMan's
-- sprite in it is 00-01, his face picture 5 (0x08014B78), Roll's image roll (08-01). (What the souls share:
-- forms/soul.)

local chip = require("@self/chip").rollsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "recovery", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("roll")),
    sprite = asset.sprite("rollsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("rollsoul") },
}

return { rollsoul = form }
