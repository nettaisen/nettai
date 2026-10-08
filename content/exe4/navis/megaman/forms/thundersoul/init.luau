--!strict
-- ThunderSoul (soul 6): EXE4's Double Soul with ThunderMan, for an elec chip given up (0x08020008[5]).
-- MegaMan's sprite in it is 00-06, his face picture 10 (0x08014B78), ThunderMan's image thunderman (08-06).
-- Its element is elec (the element byte the change sets). (What the souls share: forms/soul.)

local chip = require("@self/chip").thunsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "elec", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("thunderman")),
    sprite = asset.sprite("thundersoul"),
    element = "elec",
    buster_bonus = 0,
    weapons = soul.weapons,
    palette = 0,
    mugshot = { normal = asset.mugshot("thundersoul") },
}

return { thundersoul = form }
