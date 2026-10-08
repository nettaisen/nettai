--!strict
-- MetalSoul (soul 9): EXE4's Double Soul with MetalMan, for a metal chip given up (0x08020008[8]).
-- MegaMan's sprite in it is 00-09, his face picture 13 (0x08014B78), MetalMan's image metalman (08-09). Its weapons
-- (0x080184F0's row 9): a breaking buster on B (routine 0x0B, ./buster), damaging Metal and Null chips charged with A
-- (routine 0x0D's times: 0x0800BC78, any such chip that isn't a dimming chip), doubled charged (0x0800C47A), its fist
-- charged (routine 0x13, ./charge). Its status reset runs nothing of its own (0x0800E0A0's row 9). (What the souls
-- share: forms/soul.)

local buster = require("@self/buster")
local charge = require("@self/charge")
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
    weapons = {
        buster = buster.weapon,
        charge_shot = charge.weapon,
        a_charge = soul.chip_charge("metalsoul/charged-chip", { 70, 70, 70, 70, 70, 90 }),
    },
    charged_chips = { { family = "metal", damaging = true }, { family = "null", damaging = true } },
    charged_bonus = { doubles = true },
    palette = 0,
    mugshot = { normal = asset.mugshot("metalsoul") },
}

return { metalsoul = form }
