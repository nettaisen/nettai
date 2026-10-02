--!strict
-- The charged shot of the patch card soul-battlrs-custom
-- (`patch-cards/soul-battlrs-custom/charge`): the chip SuprVulc. The
-- original's weapon routine 0x6b (`sub_8012436`) loads the chip into the
-- attack (`loc_80126EA`, lib/weapon's `chip`) and starts what its use
-- starts.

local weapon = require("../../lib/weapon")
local chips = require("../../chips/vulcan/chips")

return define.weapon {
    id = "bn6:patch-cards/soul-battlrs-custom/charge",
    -- The charge table's row 0x6b, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 180, 170, 160, 155, 150, 120, 110, 100 },
    setup = function(navi: Object)
        return weapon.chip(navi, chips[4])
    end,
}
