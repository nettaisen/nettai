--!strict
-- GutsSoul (soul 2): EXE4's Double Soul with GutsMan, for a break chip given up (0x08020008[1]). MegaMan's
-- sprite in it is 00-02, his face picture 6 (0x08014B78), GutsMan's image gutsman (08-02). (What the souls share:
-- forms/soul.)

local chip = require("@self/chip").gutssoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "break", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("gutsman")),
    sprite = asset.sprite("gutssoul"),
    element = "null",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("gutssoul") },
}

return { gutssoul = form }
