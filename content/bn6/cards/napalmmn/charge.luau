--!strict
-- The charged shot of the patch card napalmmn
-- (`patch-cards/napalmmn/charge`): the chip BigBomb. The original's weapon
-- routine 0x65 (`sub_80123FA`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local bigbomb = require("../../chips/bigbomb/chip")

return define.weapon {
    id = "patch-cards/napalmmn/charge",
    -- The charge table's row 0x65, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 150, 140, 130, 125, 120, 90, 80, 70 },
    setup = function(navi: Object)
        return weapon.chip(navi, bigbomb)
    end,
}
