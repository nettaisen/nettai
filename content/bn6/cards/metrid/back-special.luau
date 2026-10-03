--!strict
-- The B+Back special of the patch card metrid
-- (`patch-cards/metrid/back-special`): three meteors
-- (lib/patch-cards/meteors; the original's weapon routine 0x53,
-- `loc_801232A`).

local meteors = require("../../lib/patch-cards/meteors")

return meteors.weapon {
    id = "bn6:patch-cards/metrid/back-special",
    drops = 3,
    -- The charge table's row 0x53, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 150, 140, 130 },
}
