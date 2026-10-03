--!strict
-- The charged shot of the patch card cutman (`patch-cards/cutman/charge`):
-- the chip YoYo. The original's weapon routine 0x75 (`sub_80124B8`) loads
-- the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts
-- what its use starts.

local weapon = require("../../lib/weapon")
local yoyo = require("../../chips/yoyo/chip")

return define.weapon {
    id = "bn6:patch-cards/cutman/charge",
    -- The charge table's row 0x75, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 75, 70, 0, 0, 0 },
    setup = function(navi: Object)
        return weapon.chip(navi, yoyo)
    end,
}
