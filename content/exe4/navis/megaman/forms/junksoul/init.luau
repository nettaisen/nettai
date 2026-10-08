--!strict
-- JunkSoul (soul 10): EXE4's Double Soul with JunkMan, for a summon chip given up (0x08020008[9]).
-- MegaMan's sprite in it is 00-0a, his face picture 14 (0x08014B78), JunkMan's image junkman (08-0a). (What the souls
-- share: forms/soul.)

local chip = require("@self/chip").junksoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "summon", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("junkman")),
    sprite = asset.sprite("junksoul"),
    element = "null",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("junksoul") },
}

return { junksoul = form }
