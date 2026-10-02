--!strict
-- The charged shot of the patch card thunman (`patch-cards/thunman/charge`):
-- the chip ElecSwrd. The original's weapon routine 0x64 (`sub_80123F0`)
-- loads the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and
-- starts what its use starts.

local weapon = require("../../lib/weapon")
local elecswrd = require("../../chips/elecswrd/chip")

return define.weapon {
    id = "patch-cards/thunman/charge",
    name = "ElecSwrd",
    -- The charge table's row 0x64, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 120, 110, 100, 95, 90, 150, 140, 130 },
    setup = function(navi: Object)
        return weapon.chip(navi, elecswrd)
    end,
}
