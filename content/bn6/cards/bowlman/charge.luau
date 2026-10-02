--!strict
-- The charged shot of the patch card bowlman (`patch-cards/bowlman/charge`):
-- the chip Vulcan3. The original's weapon routine 0x66 (`sub_8012404`) loads
-- the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts
-- what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/vulcan/chips")

return define.weapon {
    id = "patch-cards/bowlman/charge",
    name = "Vulcan3",
    -- The charge table's row 0x66, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 90, 80, 70, 65, 60, 150, 140, 130 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips[3])
    end,
}
