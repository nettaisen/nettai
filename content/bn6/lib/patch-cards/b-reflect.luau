--!strict
-- The B button of the patch cards (`patch-cards/b-reflect`): the chip
-- Rflectr1. The original's weapon routine 0x76 (`sub_80124C2`) loads the
-- chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts what
-- its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/rflectr/chips")

return define.weapon {
    id = "patch-cards/b-reflect",
    name = "Rflectr1",
    -- The charge table's row 0x76, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 100, 90, 80 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips[1])
    end,
}
