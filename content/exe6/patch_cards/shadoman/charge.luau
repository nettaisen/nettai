--!strict
-- The charged shot of the patch card shadoman
-- (`patch-cards/shadoman/charge`): the chip LongSwrd. The original's weapon
-- routine 0x72 (`sub_8012484`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local longswrd = require("../../chips/longswrd").longswrd

return new.weapon {
    id = "patch-cards/shadoman/charge",
    -- The charge table's row 0x72, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 90, 80, 70, 65, 60, 130, 120, 110 },
    setup = function(navi: Object)
        return weapon.chip(navi, longswrd)
    end,
}
