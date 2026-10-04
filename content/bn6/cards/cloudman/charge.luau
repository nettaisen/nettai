--!strict
-- The charged shot of the patch card cloudman
-- (`patch-cards/cloudman/charge`): the chip Thunder. The original's weapon
-- routine 0x73 (`sub_801248E`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local thunder = require("../../chips/thunder")

return define.weapon {
    id = "patch-cards/cloudman/charge",
    -- The charge table's row 0x73, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 130, 120, 110, 105, 100, 60, 55, 50 },
    setup = function(navi: Object)
        return weapon.chip(navi, thunder)
    end,
}
