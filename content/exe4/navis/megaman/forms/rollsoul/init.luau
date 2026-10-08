--!strict
-- RollSoul (soul 1): EXE4's Double Soul with Roll, for a recovery chip given up (0x08020008[0]). MegaMan's
-- sprite in it is 00-01, his face picture 5 (0x08014B78), Roll's image roll (08-01). (What the souls share:
-- forms/soul.)

local charge = require("@self/charge")
local chip = require("@self/chip").rollsoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "recovery", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("roll")),
    sprite = asset.sprite("rollsoul"),
    element = "null",
    buster_bonus = 0,
    -- Its row of 0x080184F0: the buster on B, no A charge, Roll's arrow charged (routine 0x22, ./charge).
    weapons = { buster = soul.buster, charge_shot = charge.weapon },
    palette = 0,
    mugshot = { normal = asset.mugshot("rollsoul") },
}

return { rollsoul = form }
