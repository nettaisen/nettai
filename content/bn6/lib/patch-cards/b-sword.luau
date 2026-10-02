--!strict
-- The B button of the patch cards (`patch-cards/b-sword`): the chip Sword.
-- The original's weapon routine 0x8e (`sub_8012608`) loads the chip into the
-- attack (`loc_80126EA`, lib/weapon's `chip`) and starts what its use
-- starts.

local weapon = require("../../lib/weapon")
local sword = require("../../chips/sword/chip")

return define.weapon {
    id = "patch-cards/b-sword",
    name = "Sword",
    -- The charge table's row 0x8e, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 60, 55, 50, 45, 40, 0, 0, 0 },
    setup = function(navi: Object)
        return weapon.chip(navi, sword)
    end,
}
