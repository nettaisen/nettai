--!strict
-- WoodSoul (soul 12): EXE4's Double Soul with WoodMan, for a wood chip given up (0x08020008[11]).
-- MegaMan's sprite in it is 00-0c, his face picture 16 (0x08014B78), WoodMan's image woodman (08-0c). Its element is
-- wood (0x0800C964). Its weapons (0x080184F0's row 12): the buster on B, no A charge, a tornado charged (routine 0x57,
-- ./charge). Its status reset (0x0800E294) folds into each Wood chip of the hand from its cursor on the damaging Null
-- chip after it, and so does the custom screen as OK builds a hand in it (0x0801F226, which leaves each slot's Atk+
-- bonus and marks where they were). A hit's status doesn't take on it (0x08013904 skips 0x08013470); grass heals it a point every other
-- tick (rules/souls, 0x08012FB6). (What the souls share: forms/soul.)

local charge = require("@self/charge")
local chip = require("@self/chip").woodsoul
local soul = require("./soul")

-- 0x0800E294 (`whole`: on the hand) and 0x0801F226 (on the custom screen's list): from the hand's cursor on, a Wood
-- chip takes in the damaging Null chip after it (its damage and bonuses), which leaves the hand (the chips after it
-- moving up); then the chip after the Wood chip is looked at (one Null chip a Wood chip).
local function fold(side: number, whole: boolean)
    local i = battle.hand_cursor(side)
    while true do
        local c = battle.hand_chip(side, i)
        if c == nil then
            break
        end
        if c.family == "wood" then
            local after = battle.hand_chip(side, i + 1)
            if after == nil then
                break
            end
            if after.family == "null" and battle.hand_chip_damages(side, i + 1) then
                battle.fold_hand_chip(side, i, whole)
            end
        end
        i += 1
    end
end

local form: FormDef = {
    soul = { family = "wood", turns = 3 },
    custom = soul.custom {
        built = function(side: number)
            fold(side, false)
        end,
    },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("woodman")),
    sprite = asset.sprite("woodsoul"),
    element = "wood",
    buster_bonus = 0,
    weapons = { buster = soul.buster, charge_shot = charge.weapon },
    traits = { "status_immune" },
    reset = function(navi: Object)
        fold(navi.alliance, true)
    end,
    palette = 0,
    mugshot = { normal = asset.mugshot("woodsoul") },
}

return { woodsoul = form }
