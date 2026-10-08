--!strict
-- SearchSoul (soul 4): EXE4's Double Soul with SearchMan, for an invisible chip given up (0x08020008[3]).
-- MegaMan's sprite in it is 00-04, his face picture 8 (0x08014B78), SearchMan's image searchman (08-04). Its weapons
-- (0x080184F0's row 4): the buster on B, no A charge, the scope's shots charged (routine 0x0A, ./charge). Its status
-- reset (0x0800E13E) reveals the other side's hidden navi (./reveal). (What the souls share: forms/soul.)

local charge = require("@self/charge")
local chip = require("@self/chip").srchsoul
local reveal = require("@self/reveal")
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
    weapons = { buster = soul.buster, charge_shot = charge.weapon },
    -- 0x0800E13E.
    reset = function(navi: Object)
        reveal.spawn(navi)
    end,
    palette = 0,
    mugshot = { normal = asset.mugshot("searchsoul") },
}

return { searchsoul = form }
