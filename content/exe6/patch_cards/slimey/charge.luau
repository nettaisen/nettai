--!strict
-- The charged shot of the patch card slimey (`patch-cards/slimey/charge`):
-- the chip PanlGrab. The original's weapon routine 0x57 (`sub_801235A`)
-- loads the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and
-- starts what its use starts.

local weapon = require("../../lib/weapon")
local panlgrab = require("../../chips/panlgrab").panlgrab

return new.weapon {
    id = "patch-cards/slimey/charge",
    -- The charge table's row 0x57, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 180, 170, 160, 155, 150, 180, 170, 160 },
    setup = function(navi: Object)
        return weapon.chip(navi, panlgrab)
    end,
}
