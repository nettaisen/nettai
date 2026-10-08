--!strict
-- MetalSoul (soul 9): EXE4's Double Soul with MetalMan, for a metal chip given up (0x08020008[8]).
-- MegaMan's sprite in it is 00-09, his face picture 13 (0x08014B78), MetalMan's image metalman (08-09). (What the souls
-- share: forms/soul.)

local chip = require("@self/chip").metlsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "metal", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("metalman")),
    sprite = asset.sprite("metalsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("metalsoul") },
}

return { metalsoul = form }
