--!strict
-- The charged shot of the patch card diveman (`patch-cards/diveman/charge`):
-- the chip WideSht. The original's weapon routine 0x91 (`sub_8012628`) loads
-- the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts
-- what its use starts.

local weapon = require("../../lib/weapon")
local widesht = require("../../chips/widesht")

return define.weapon {
    id = "patch-cards/diveman/charge",
    -- The charge table's row 0x91, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 150, 140, 130, 125, 120, 110, 100, 90 },
    setup = function(navi: Object)
        return weapon.chip(navi, widesht)
    end,
}
