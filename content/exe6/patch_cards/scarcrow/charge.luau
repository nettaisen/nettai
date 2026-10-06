--!strict
-- The charged shot of the patch card scarcrow
-- (`patch-cards/scarcrow/charge`): the chip DolThdr1. The original's weapon
-- routine 0x59 (`sub_801236E`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local dolthdr_chips = require("../../chips/dolthdr")

return new.weapon {
    id = "patch-cards/scarcrow/charge",
    -- The charge table's row 0x59, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 130, 120, 110, 105, 100, 50, 45, 40 },
    setup = function(navi: Object)
        return weapon.chip(navi, dolthdr_chips.dolthdr1)
    end,
}
