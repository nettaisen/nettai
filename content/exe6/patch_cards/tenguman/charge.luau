--!strict
-- The charged shot of the patch card tenguman
-- (`patch-cards/tenguman/charge`): the chip WindRack. The original's weapon
-- routine 0x69 (`sub_8012422`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local windrack = require("../../chips/windrack")

return define.weapon {
    id = "patch-cards/tenguman/charge",
    -- The charge table's row 0x69, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 75, 70, 200, 190, 180 },
    setup = function(navi: Object)
        return weapon.chip(navi, windrack)
    end,
}
