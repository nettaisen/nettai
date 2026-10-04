--!strict
-- The B+Back special of the patch card darkmech
-- (`patch-cards/darkmech/back-special`): the chip MchnSwrd. The original's
-- weapon routine 0x89 (`sub_8012572`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local mchnswrd = require("../../chips/mchnswrd")

return define.weapon {
    id = "patch-cards/darkmech/back-special",
    -- The charge table's row 0x89, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 200, 190, 180 },
    setup = function(navi: Object)
        return weapon.chip(navi, mchnswrd)
    end,
}
