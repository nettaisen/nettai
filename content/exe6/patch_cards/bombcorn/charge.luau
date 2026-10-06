--!strict
-- The charged shot of the patch card bombcorn
-- (`patch-cards/bombcorn/charge`): the chip CornSht1. The original's weapon
-- routine 0x62 (`sub_80123DC`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local cornsht_chips = require("../../chips/cornsht")

return new.weapon {
    id = "patch-cards/bombcorn/charge",
    -- The charge table's row 0x62, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 80, 70, 60, 55, 50, 130, 120, 110 },
    setup = function(navi: Object)
        return weapon.chip(navi, cornsht_chips.cornsht1)
    end,
}
