--!strict
-- The charged shot of the patch card canguard
-- (`patch-cards/canguard/charge`): the chip Cannon. The original's weapon
-- routine 0x61 (`sub_80123D2`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/cannon/chips")

return define.weapon {
    id = "bn6:patch-cards/canguard/charge",
    -- The charge table's row 0x61, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 60, 50, 40, 35, 30, 80, 70, 60 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips.cannon)
    end,
}
