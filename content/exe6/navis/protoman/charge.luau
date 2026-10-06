--!strict
-- ProtoMan's charged attack (`protoman/charge`): the original's weapon
-- routine 0x32 (`sub_801206E`). It is WideSwrd's slash: the routine loads
-- the chip into the attack (`loc_80126EA`, which doesn't spend one), then
-- gives it the charge's damage and counter byte.

local weapon = require("../../lib/weapon")
local wideswrd = require("../../chips/wideswrd").wideswrd

local charge = {}

-- The attack's element byte: WideSwrd's (Null, with its family's Sword
-- bit), and the hits' counter byte.
local SWORD, HIT_PARAM = 0x80, 0x8A

-- `sub_801206E`: WideSwrd as the attack's chip (its lockout, none; no
-- bonus; not charged), with 60 damage and 20 more per point of buster
-- Attack (up to 5).
local function setup(navi: Object): Action
    navi.attack_chip = wideswrd
    navi.attack_element = SWORD
    navi.attack_lockout = 0
    navi.extra = 0
    navi.charged = 0
    navi.attack_damage = weapon.charge_damage(navi, 0x3C, 0x14)
    navi.hit_param = HIT_PARAM
    return assert(wideswrd.action)
end

charge.weapon = new.weapon {
    id = "protoman/charge",
    -- The charge table's row 0x32, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 40, 35, 30, 25, 20, 50, 50, 50 },
    setup = setup,
}

return charge
