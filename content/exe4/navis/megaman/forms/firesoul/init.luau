--!strict
-- FireSoul (soul 5): EXE4's Double Soul with FireMan, for a fire chip given up (0x08020008[4]).
-- MegaMan's sprite in it is 00-05, his face picture 9 (0x08014B78), FireMan's image fireman (08-05), which holds the
-- flame on his head as it comes (attachment row 0x12: 10-08). Its element is fire (0x0800C964); it wears its flames
-- (0x08018068's 0x10: 10-00, forms/soul's overlay). Its weapons (0x080184F0's row 5): the buster on B, fire chips
-- charged with A (routine 0x66, ./charged_chip), the flame charged (routine 0x0C, ./charge). Its status reset
-- (0x0800E11C) turns the middle row and four more panels grass (./grass); lava heals it (rules/panels, 0x0801313E).
-- (What the souls share: forms/soul.)

local attachment = require("../../../objects/attachment")
local charge = require("@self/charge")
local charged_chip = require("@self/charged_chip")
local chip = require("@self/chip").firesoul
local grass = require("@self/grass")
local soul = require("./soul")

local overlay = soul.overlay(asset.sprite("firesoul-flame"))

local form: FormDef = {
    soul = { family = "fire", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("fireman"), attachment.look { sprite = asset.sprite("fireman-flame"), palette = 0 }),
    sprite = asset.sprite("firesoul"),
    element = "fire",
    buster_bonus = 0,
    weapons = { buster = soul.buster, charge_shot = charge.weapon, a_charge = charged_chip.weapon },
    -- 0x0800BC78: any fire chip.
    charged_chips = { { family = "fire", damaging = false } },
    put_on = overlay.put_on,
    take_off = overlay.take_off,
    -- 0x0800E11C.
    reset = function(navi: Object)
        grass.spawn(navi)
    end,
    palette = 0,
    mugshot = { normal = asset.mugshot("firesoul") },
}

return { firesoul = form }
