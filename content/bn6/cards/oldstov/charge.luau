--!strict
-- The charged shot of the patch card oldstov (`patch-cards/oldstov/charge`):
-- the chip FireBrn1. The original's weapon routine 0x78 (`sub_80124CC`)
-- loads the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and
-- starts what its use starts.

local weapon = require("../../lib/weapon")
local firebrn_chips = require("../../chips/firebrn")

return define.weapon {
    id = "patch-cards/oldstov/charge",
    -- The charge table's row 0x78, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 75, 70, 100, 90, 80 },
    setup = function(navi: Object)
        return weapon.chip(navi, firebrn_chips.firebrn1)
    end,
}
