--!strict
-- FireSoul (soul 5, Red Sun's): EXE4's Double Soul with FireMan, for a fire chip given up (0x08020008[4]).
-- MegaMan's sprite in it is 00-05, his face picture 9 (0x08014B78), FireMan's image fireman (08-05).
-- Its element is fire (the element byte the change sets). (What the souls share: forms/soul.)

local chip = require("@self/chip").firesoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "fire", turns = 3 },
    chip = chip,
    version = "redsun",
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("fireman")),
    sprite = asset.sprite("firesoul"),
    element = "fire",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("firesoul") },
}

return { firesoul = form }
