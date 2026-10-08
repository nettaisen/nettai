--!strict
-- SearchSoul (soul 4): EXE4's Double Soul with SearchMan, for an invisible chip given up (0x08020008[3]).
-- MegaMan's sprite in it is 00-04, his face picture 8 (0x08014B78), SearchMan's image searchman (08-04). (What the
-- souls share: forms/soul.)

local chip = require("@self/chip").srchsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "invisible", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("searchman")),
    sprite = asset.sprite("searchsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("searchsoul") },
}

return { searchsoul = form }
