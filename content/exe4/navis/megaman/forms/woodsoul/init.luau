--!strict
-- WoodSoul (soul 12, Blue Moon's): EXE4's Double Soul with WoodMan, for a wood chip given up (0x08020008[11]).
-- MegaMan's sprite in it is 00-0c, his face picture 16 (0x08014B78), WoodMan's image woodman (08-0c).
-- Its element is wood (the element byte the change sets). (What the souls share: forms/soul.)

local chip = require("@self/chip").woodsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "wood", turns = 3 },
    chip = chip,
    version = "bluemoon",
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("woodman")),
    sprite = asset.sprite("woodsoul"),
    element = "wood",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("woodsoul") },
}

return { woodsoul = form }
