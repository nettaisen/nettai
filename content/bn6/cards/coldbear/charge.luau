--!strict
-- The charged shot of the patch card coldbear
-- (`patch-cards/coldbear/charge`): the chip IceCube. The original's weapon
-- routine 0x55 (`sub_8012346`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/rockcube/chips")

return define.weapon {
    id = "bn6:patch-cards/coldbear/charge",
    -- The charge table's row 0x55, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 180, 170, 160, 155, 150, 100, 90, 80 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips.icecube)
    end,
}
