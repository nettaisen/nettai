--!strict
-- The charged shot of the patch card kilby (`patch-cards/kilby/charge`): the
-- chip Lance. The original's weapon routine 0x60 (`sub_80123C8`) loads the
-- chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts what
-- its use starts.

local weapon = require("../../lib/weapon")
local lance = require("../../chips/lance/chip")

return define.weapon {
    id = "bn6:patch-cards/kilby/charge",
    -- The charge table's row 0x60, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 120, 110, 100, 95, 90, 60, 50, 40 },
    setup = function(navi: Object)
        return weapon.chip(navi, lance)
    end,
}
