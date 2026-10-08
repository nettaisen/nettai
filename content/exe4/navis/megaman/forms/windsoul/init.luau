--!strict
-- WindSoul (soul 3): EXE4's Double Soul with WindMan, for a wind chip given up (0x08020008[2]). MegaMan's
-- sprite in it is 00-03, his face picture 7 (0x08014B78), WindMan's image windman (08-03). (What the souls share:
-- forms/soul.)

local attachment = require("../../../objects/attachment")
local chip = require("@self/chip").windsoul
local collision = require("../../../rules/collision")
local soul = require("./soul")
local wind = require("../../../objects/wind")

-- What its status reset gives the hand's wind chips.
local BONUS = 10

local form: FormDef = {
    soul = { family = "wind", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    -- WindMan's image holds his swirl as it comes (attachment row 0x34: 10-28, palette 0).
    image = soul.image(asset.sprite("windman"), attachment.look { sprite = asset.sprite("windman-swirl"), palette = 0 }),
    sprite = asset.sprite("windsoul"),
    element = "null",
    buster_bonus = 0,
    weapons = soul.weapons,
    -- 0x0800E0F4: air and float shoes (flags 0x10, 0x20), a floating navi's body (row 0x10; EXE4's retype sets what it
    -- is alone), its wind (objects/wind: a navi's, pulling), which the next status reset ends (AIData +0x5C), and 10
    -- more on each damaging Wind chip of the hand from its cursor on (0x0800E208).
    reset = function(navi: Object)
        navi:set_status("air_shoe", true)
        navi:set_status("float_shoe", true)
        navi:reset_collision_types(collision.floating_navi, collision.navi_target, 0)
        navi.reset_linked_object = wind.navi(navi)
        local side = navi.alliance
        local i = battle.hand_cursor(side)
        while true do
            local c = battle.hand_chip(side, i)
            if c == nil then
                break
            end
            if battle.hand_chip_damages(side, i) and c.family == "wind" then
                battle.add_hand_charge_bonus(side, i, BONUS)
            end
            i += 1
        end
    end,
    palette = 0,
    mugshot = { normal = asset.mugshot("windsoul") },
}

return { windsoul = form }
