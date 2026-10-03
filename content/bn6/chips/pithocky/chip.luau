--!strict
-- PitHocky (AirHocky's Program Advance): AirHocky's flick
-- (chips/airhocky/flick.luau) of a puck that bounces for 18 steps,
-- leaving a trail.

local flick = require("../airhocky/flick")
local puck = require("../airhocky/puck")
local airhocky = require("../airhocky/chip")

return define.chip {
    id = "bn6:pithocky",
    codes = {},
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 5,
    lockout = 0,
    flags = { "has_damage", "library" },
    library = { number = 341, index = 22, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 26, code_run = { chip = airhocky, count = 3 } } },
    action = flick.action { id = "bn6:pithocky/action", puck = puck.variants.pithocky, down = true },
}
