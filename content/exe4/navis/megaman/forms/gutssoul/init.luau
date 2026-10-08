--!strict
-- GutsSoul (soul 2): EXE4's Double Soul with GutsMan, for a break chip given up (0x08020008[1]). MegaMan's
-- sprite in it is 00-02, his face picture 6 (0x08014B78), GutsMan's image gutsman (08-02). Its weapons (0x080184F0's
-- row 2): the buster on B, no A charge, GutsMan's fist charged (routine 0x24, ./charge); six quick B presses fire its
-- machine gun (0x0800BE50, ./machine_gun). Its status reset (0x0800E1CA) gives each damaging Null and Break chip of the
-- hand from its cursor on 30 more. (What the souls share: forms/soul.)

local charge = require("@self/charge")
local chip = require("@self/chip").gutssoul
local soul = require("./soul")

-- The status reset's bonus, and the families it goes to.
local BONUS = 30
local function bonus_family(family: string?): boolean
    return family == "null" or family == "break"
end

local form: FormDef = {
    soul = { family = "break", turns = 3 },
    chip = chip,
    change = chip.action,
    revert = soul.revert,
    image = soul.image(asset.sprite("gutsman")),
    sprite = asset.sprite("gutssoul"),
    element = "null",
    buster_bonus = 0,
    weapons = { buster = soul.buster, charge_shot = charge.weapon },
    -- Six B presses, each within 10 ticks of the last, ask for the machine gun (request 0x20: the role
    -- `actions.forced_charged_shot`).
    rapid_presses = { presses = 6, window = 10 },
    -- 0x0800E1CA.
    reset = function(navi: Object)
        local side = navi.alliance
        local i = battle.hand_cursor(side)
        while true do
            local c = battle.hand_chip(side, i)
            if c == nil then
                break
            end
            if battle.hand_chip_damages(side, i) and bonus_family(c.family) then
                battle.add_hand_charge_bonus(side, i, BONUS)
            end
            i += 1
        end
    end,
    palette = 0,
    mugshot = { normal = asset.mugshot("gutssoul") },
}

return { gutssoul = form }
