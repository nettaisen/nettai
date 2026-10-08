--!strict
-- ProtoSoul (soul 7): EXE4's Double Soul with ProtoMan, for a sword chip given up (0x08020008[6]).
-- MegaMan's sprite in it is 00-07, his face picture 11 (0x08014B78), ProtoMan's image protoman (08-07). Its weapons
-- (0x080184F0's row 7): the buster on B, sword chips charged with A (routine 0x0F's times: 0x0800BC78, any Sword chip
-- that isn't a dimming chip; every one deals damage), doubled charged (0x0800C47A), WideSwrd charged (routine 0x3E,
-- ./charge); its status reset gives it its B+Left special (0x0800E12C: routine 0x6A, ./reflect). Its swords swing
-- blade 13 and hit with its own modifier (lib/swords). (What the souls share: forms/soul.)

local charge = require("@self/charge")
local chip = require("@self/chip").protsoul
local reflect = require("@self/reflect")
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
    weapons = {
        buster = soul.buster,
        charge_shot = charge.weapon,
        a_charge = soul.chip_charge("protosoul/charged-chip", { 30, 30, 30, 30, 30, 80 }),
    },
    charged_chips = { { family = "sword", damaging = true } },
    charged_bonus = { doubles = true },
    -- 0x0800E12C: the B+Left special (AIData +0x00, which the weapon load leaves in a soul).
    reset = function(navi: Object)
        navi.back_special_weapon = reflect.weapon
    end,
    palette = 0,
    mugshot = { normal = asset.mugshot("protosoul") },
}

return { protosoul = form }
