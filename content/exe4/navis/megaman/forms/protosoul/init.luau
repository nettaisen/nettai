--!strict
-- ProtoSoul (soul 7): EXE4's Double Soul with ProtoMan, for a sword chip given up (0x08020008[6]).
-- MegaMan's sprite in it is 00-07, his face picture 11 (0x08014B78), ProtoMan's image protoman (08-07). (What the souls
-- share: forms/soul.)

local chip = require("@self/chip").protsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "sword", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("protoman")),
    sprite = asset.sprite("protosoul"),
    element = "null",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("protosoul") },
}

return { protosoul = form }
