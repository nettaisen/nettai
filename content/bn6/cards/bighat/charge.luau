--!strict
-- The charged shot of the patch card bighat (`patch-cards/bighat/charge`):
-- the chip FlshBom1. The original's weapon routine 0x58 (`sub_8012364`)
-- loads the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and
-- starts what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/flshbom/chips")

return define.weapon {
    id = "patch-cards/bighat/charge",
    -- The charge table's row 0x58, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 180, 170, 160, 155, 150, 130, 120, 110 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips[1])
    end,
}
