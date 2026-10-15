--!strict
-- NumberSoul (soul 8): EXE4's Double Soul with NumberMan, for a plus chip given up (0x08020008[7]).
-- MegaMan's sprite in it is 00-08, his face picture 12 (0x08014B78), NumberMan's image numberman (08-08), which holds
-- his face as it comes (attachment row 0x14: 10-06). It wears its overlay (0x08018068's 0x110: 10-01, forms/soul's
-- overlay). Its weapons (0x080184F0's row 8): the buster on B, no A charge, a die charged (routine 0x11, ./charge).
-- Its status reset (0x0800E194) gives each damaging Null chip of the hand from its cursor on 10 more, and so does
-- the custom screen as OK builds a hand in it (0x0801F29A). Its custom screen deals ten chips (rules/souls/custom's
-- hand size, 0x0801DC92). (What the souls share: forms/soul.)

local attachment = require("../../../objects/attachment")
local charge = require("@self/charge")
local chip = require("@self/chip").numsoul
local soul = require("./soul")

local OVERLAY = asset.sprite("numbersoul-overlay")
local overlay = soul.overlay(OVERLAY)

-- The status reset's bonus, and the custom screen's.
local BONUS = 10

-- 0x0800E194, and 0x0801F29A's for soul 8 on the hand installed.
local function hand(side: number)
    local i = battle.hand_cursor(side)
    while true do
        local c = battle.hand_chip(side, i)
        if c == nil then
            break
        end
        if battle.hand_chip_damages(side, i) and c.family == "null" then
            battle.add_hand_charge_bonus(side, i, BONUS)
        end
        i += 1
    end
end

local form: FormDef = {
    -- The version whose saves own it (rules/builds' groups).
    version = "bluemoon",
    soul = { family = "plus", turns = 3 },
    custom = soul.custom { built = hand },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("numberman"), attachment.look { sprite = asset.sprite("numberman-face"), palette = 0 }),
    sprite = asset.sprite("numbersoul"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = soul.buster, charge_shot = charge.weapon },
    wears = OVERLAY,
    put_on = overlay.put_on,
    take_off = overlay.take_off,
    reset = function(navi: Object)
        hand(navi.alliance)
    end,
    palette = 0,
    mugshot = { normal = asset.mugshot("numbersoul") },
}

return { numbersoul = form }
