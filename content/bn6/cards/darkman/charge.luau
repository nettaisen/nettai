--!strict
-- The charged shot of the patch card darkman (`patch-cards/darkman/charge`):
-- the chip VDoll. The original's weapon routine 0x67 (`sub_801240E`) loads
-- the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts
-- what its use starts.

local weapon = require("../../lib/weapon")
local vdoll = require("../../chips/vdoll/chip")

return define.weapon {
    id = "patch-cards/darkman/charge",
    name = "VDoll",
    -- The charge table's row 0x67, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 150, 140, 130, 125, 120, 100, 90, 80 },
    setup = function(navi: Object)
        return weapon.chip(navi, vdoll)
    end,
}
