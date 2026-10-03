--!strict
-- The charged shot of the patch card handy (`patch-cards/handy/charge`): the
-- chip TimeBom1. The original's weapon routine 0x5e (`sub_80123A0`) loads
-- the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts
-- what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/timebom/chips")

return define.weapon {
    id = "bn6:patch-cards/handy/charge",
    -- The charge table's row 0x5e, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 150, 140, 130, 125, 120, 100, 90, 80 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips.timebom1)
    end,
}
