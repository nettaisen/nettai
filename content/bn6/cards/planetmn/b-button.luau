--!strict
-- The B button of the patch card planetmn (`patch-cards/planetmn/b-button`):
-- the chip CrakShot. The original's weapon routine 0x81 (`sub_801251C`)
-- loads the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and
-- starts what its use starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/crakshot")

return define.weapon {
    id = "patch-cards/planetmn/b-button",
    -- The charge table's row 0x81, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 60, 55, 50, 45, 40, 100, 90, 80 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips[1])
    end,
}
