--!strict
-- The B button of the patch card mettfire (`patch-cards/mettfire/b-button`):
-- a meteor (lib/patch_cards/meteors; the original's weapon routine 0x52,
-- `sub_8012326`).

local meteors = require("../../lib/patch_cards/meteors")

return meteors.weapon {
    id = "patch-cards/mettfire/b-button",
    drops = 1,
    -- The charge table's row 0x52, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 0, 0, 0 },
}
