--!strict
-- The charged shot of the patch card kilplant
-- (`patch-cards/kilplant/charge`): the chip GrasSeed. The original's weapon
-- routine 0x56 (`sub_8012350`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local grasseed = require("../../chips/grasseed/chip")

return define.weapon {
    id = "patch-cards/kilplant/charge",
    name = "GrasSeed",
    -- The charge table's row 0x56, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 85, 70, 180, 170, 160 },
    setup = function(navi: Object)
        return weapon.chip(navi, grasseed)
    end,
}
