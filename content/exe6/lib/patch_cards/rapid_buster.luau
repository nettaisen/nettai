--!strict
-- The patch cards' rapid buster (`patch-cards/rapid-buster`, the original's
-- weapon routine 0x92, `sub_8012124`, routine 0x39's with its own charge
-- row; Bass BX's charged shot, and Gregar's Bass-Cross MegaMan card's): the
-- rapid buster (lib/rapid_buster: nine charged shots, 5 damage for each
-- point of Attack plus one).

local rapid_buster = require("../rapid_buster")

return new.weapon {
    id = "patch-cards/rapid-buster",
    -- The charge table's row 0x92, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 110, 100, 90, 80, 70, 140, 130, 120 },
    setup = rapid_buster.setup,
}
