--!strict
-- The charged shot of the patch card slashman
-- (`patch-cards/slashman/charge`): the chip WideSwrd. The original's weapon
-- routine 0x7c (`sub_80124E0`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local wideswrd = require("../../chips/wideswrd")

return define.weapon {
    id = "patch-cards/slashman/charge",
    -- The charge table's row 0x7c, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 70, 60, 60, 55, 50 },
    setup = function(navi: Object)
        return weapon.chip(navi, wideswrd)
    end,
}
