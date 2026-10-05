--!strict
-- The charged shot of the patch card iceman (`patch-cards/iceman/charge`):
-- the chip IceSeed. The original's weapon routine 0x5b (`sub_8012382`) loads
-- the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts
-- what its use starts.

local weapon = require("../../lib/weapon")
local iceseed = require("../../chips/iceseed")

return define.weapon {
    id = "patch-cards/iceman/charge",
    -- The charge table's row 0x5b, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 150, 140, 130, 125, 120, 130, 120, 110 },
    setup = function(navi: Object)
        return weapon.chip(navi, iceseed)
    end,
}
