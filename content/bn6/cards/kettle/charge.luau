--!strict
-- The charged shot of the patch card kettle (`patch-cards/kettle/charge`):
-- the chip Spreadr1. The original's weapon routine 0x5a (`sub_8012378`)
-- loads the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and
-- starts what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/spreadr")

return define.weapon {
    id = "patch-cards/kettle/charge",
    -- The charge table's row 0x5a, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 50, 45, 40, 35, 30, 150, 140, 130 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips.spreadr1)
    end,
}
