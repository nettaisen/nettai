--!strict
-- The charged shot of the patch card pulsbulb
-- (`patch-cards/pulsbulb/charge`): the chip ElcPuls1. The original's weapon
-- routine 0x7a (`sub_80124D6`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/elcpuls")

return define.weapon {
    id = "patch-cards/pulsbulb/charge",
    -- The charge table's row 0x7a, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 140, 130, 120, 115, 110, 100, 90, 80 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips.elcpuls1)
    end,
}
