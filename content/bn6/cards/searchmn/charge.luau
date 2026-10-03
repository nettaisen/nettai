--!strict
-- The charged shot of the patch card searchmn
-- (`patch-cards/searchmn/charge`): the chip CircGun. The original's weapon
-- routine 0x84 (`sub_8012536`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local circgun = require("../../chips/circgun/chip")

return define.weapon {
    id = "bn6:patch-cards/searchmn/charge",
    -- The charge table's row 0x84, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 180, 170, 160, 155, 150, 120, 110, 100 },
    setup = function(navi: Object)
        return weapon.chip(navi, circgun)
    end,
}
