--!strict
-- The charged shot of the patch card megaman-zero
-- (`patch-cards/megaman-zero/charge`): the chip Z Saver. The original's
-- weapon routine 0x6e (`sub_801245A`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local z_saver = require("../../chips/z-saver")

return define.weapon {
    id = "patch-cards/megaman-zero/charge",
    -- The charge table's row 0x6e, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 160, 150, 140, 135, 130, 100, 90, 80 },
    setup = function(navi: Object)
        return weapon.chip(navi, z_saver)
    end,
}
