--!strict
-- The charged shot of the patch card grndman (`patch-cards/grndman/charge`):
-- the chip DrilArm. The original's weapon routine 0x85 (`sub_8012540`) loads
-- the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts
-- what its use starts.

local weapon = require("../../lib/weapon")
local drilarm = require("../../chips/drilarm/chip")

return define.weapon {
    id = "bn6:patch-cards/grndman/charge",
    -- The charge table's row 0x85, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 120, 110, 100, 95, 90, 0, 0, 0 },
    setup = function(navi: Object)
        return weapon.chip(navi, drilarm)
    end,
}
