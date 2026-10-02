--!strict
-- The charged shot of the patch card dark-megaman
-- (`patch-cards/dark-megaman/charge`): the chip DrkSword. The original's
-- weapon routine 0x8a (`sub_801257C`) loads the chip into the attack
-- (`loc_80126EA`, lib/weapon's `chip`) and starts what its use starts.

local weapon = require("../../lib/weapon")
local drksword = require("../../chips/drksword/chip")

return define.weapon {
    id = "patch-cards/dark-megaman/charge",
    name = "DrkSword",
    -- The charge table's row 0x8a, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 200, 190, 180, 175, 170, 0, 0, 0 },
    setup = function(navi: Object)
        return weapon.chip(navi, drksword)
    end,
}
