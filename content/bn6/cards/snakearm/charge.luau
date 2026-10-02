--!strict
-- The charged shot of the patch card snakearm
-- (`patch-cards/snakearm/charge`): the chip SandWrm1. The original's weapon
-- routine 0x63 (`sub_80123E6`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/sandwrm/chips")

return define.weapon {
    id = "patch-cards/snakearm/charge",
    -- The charge table's row 0x63, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 130, 120, 110, 105, 100, 120, 110, 100 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips[1])
    end,
}
