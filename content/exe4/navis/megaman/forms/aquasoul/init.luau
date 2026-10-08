--!strict
-- AquaSoul (soul 11): EXE4's Double Soul with AquaMan, for an aqua chip given up (0x08020008[10]).
-- MegaMan's sprite in it is 00-0b, his face picture 15 (0x08014B78), AquaMan's image aquaman (08-0b), which holds the
-- spout on his head as it comes (attachment row 0x13: 10-0c). Its element is aqua (0x0800C964). Its weapons
-- (0x080184F0's row 11): the buster on B, aqua chips charged with A (routine 0x15's times: 0x0800BC78, any aqua chip
-- that isn't a dimming chip; every one deals damage), doubled charged (0x0800C47A), Bubbler's spreader charged (routine
-- 0x23, ./charge). Its status reset runs nothing of its own (0x0800E0A0's row 11). (What the souls share: forms/soul.)

local attachment = require("../../../objects/attachment")
local charge = require("@self/charge")
local chip = require("@self/chip").aquasoul
local soul = require("./soul")

local form: FormDef = {
    soul = { family = "aqua", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("aquaman"), attachment.look { sprite = asset.sprite("aquaman-spout"), palette = 0 }),
    sprite = asset.sprite("aquasoul"),
    element = "aqua",
    buster_bonus = 0,
    weapons = {
        buster = soul.buster,
        charge_shot = charge.weapon,
        a_charge = soul.chip_charge("aquasoul/charged-chip", { 20, 20, 20, 20, 20, 60 }),
    },
    charged_chips = { { family = "aqua", damaging = true } },
    charged_bonus = { doubles = true },
    palette = 0,
    mugshot = { normal = asset.mugshot("aquasoul") },
}

return { aquasoul = form }
