--!strict
-- MetrKnuk's dimming controller (the original's effect object #0x76,
-- `sub_80E8B00`; docs/engine/dimming-chip-effects.md §18): the shared
-- controller (@common/metrknuk/controller), BN6's kind: the usual dimming
-- phases, with an effect that throws 16 fists (./fist) at the enemy's
-- area, 8 ticks apart: the first at a random panel an enemy stands on, the
-- others at random panels around the enemies (never the last one twice in
-- a row), or of the enemy's area when no enemy stands.

local common = require("@common/metrknuk/controller")
local fist = require("./fist")

return common.make {
    id = "bn6:metrknuk/controller",
    fist = fist.spawn,
    first_on_body = true,
    area_fallback = true,
    fists_take_element = true,
}
