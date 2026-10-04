--!strict
-- The charged shot of the patch card judgeman
-- (`patch-cards/judgeman/charge`): the chip GrabBnsh. The original's weapon
-- routine 0x6a (`sub_801242C`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/grabbnsh/chips")

return define.weapon {
    id = "patch-cards/judgeman/charge",
    -- The charge table's row 0x6a, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 200, 190, 180, 175, 170, 180, 170, 160 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips.grabbnsh)
    end,
}
