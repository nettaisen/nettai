--!strict
-- The charged shot of the patch card volcano (`patch-cards/volcano/charge`):
-- the chip BlkBomb. The original's weapon routine 0x87 (`sub_8012552`) loads
-- the chip into the attack (`loc_80126EA`, lib/weapon's `chip`) and starts
-- what its use starts.

local weapon = require("../../lib/weapon")
local blkbomb = require("../../chips/blkbomb/chip")

return define.weapon {
    id = "patch-cards/volcano/charge",
    name = "BlkBomb",
    -- The charge table's row 0x87, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 160, 150, 140, 135, 130, 0, 0, 0 },
    setup = function(navi: Object)
        return weapon.chip(navi, blkbomb)
    end,
}
