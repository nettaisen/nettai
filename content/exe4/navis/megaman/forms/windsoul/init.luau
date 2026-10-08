--!strict
-- WindSoul (soul 3): EXE4's Double Soul with WindMan, for a wind chip given up (0x08020008[2]). MegaMan's
-- sprite in it is 00-03, his face picture 7 (0x08014B78), WindMan's image windman (08-03). (What the souls share:
-- forms/soul.)

local chip = require("@self/chip").windsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "wind", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("windman")),
    sprite = asset.sprite("windsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("windsoul") },
}

return { windsoul = form }
