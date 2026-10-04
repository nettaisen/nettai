--!strict
-- The charged shot of the patch card canodumb
-- (`patch-cards/canodumb/charge`): the chip M-Cannon. The original's weapon
-- routine 0x54 (`sub_801233C`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/cannon")

return define.weapon {
    id = "patch-cards/canodumb/charge",
    -- The charge table's row 0x54, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 150, 140, 130, 125, 120, 180, 170, 160 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips.m_cannon)
    end,
}
