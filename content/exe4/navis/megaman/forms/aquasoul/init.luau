--!strict
-- AquaSoul (soul 11, Blue Moon's): EXE4's Double Soul with AquaMan, for an aqua chip given up (0x08020008[10]).
-- MegaMan's sprite in it is 00-0b, his face picture 15 (0x08014B78), AquaMan's image aquaman (08-0b).
-- Its element is aqua (the element byte the change sets). (What the souls share: forms/soul.)

local chip = require("@self/chip").aquasoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "aqua", turns = 3 },
    chip = chip,
    version = "bluemoon",
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("aquaman")),
    sprite = asset.sprite("aquasoul"),
    element = "aqua",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("aquasoul") },
}

return { aquasoul = form }
