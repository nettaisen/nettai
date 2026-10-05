--!strict
-- The charged shot of the patch card django (`patch-cards/django/charge`):
-- the chip GunDelS1. The original's weapon routine 0x6c (`sub_8012440`)
-- loads the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and
-- starts what its use starts.

local weapon = require("../../lib/weapon")
local gundels_chips = require("../../chips/gundels")

return define.weapon {
    id = "patch-cards/django/charge",
    -- The charge table's row 0x6c, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 120, 110, 100, 95, 90, 210, 200, 190 },
    setup = function(navi: Object)
        return weapon.chip(navi, gundels_chips.gundels1)
    end,
}
