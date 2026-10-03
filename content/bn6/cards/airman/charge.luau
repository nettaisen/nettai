--!strict
-- The charged shot of the patch card airman (`patch-cards/airman/charge`):
-- the chip Tornado. The original's weapon routine 0x7f (`sub_80124F4`) loads
-- the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts
-- what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/tornado/chips")

return define.weapon {
    id = "bn6:patch-cards/airman/charge",
    -- The charge table's row 0x7f, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 120, 115, 110, 105, 100, 60, 55, 50 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips[1])
    end,
}
