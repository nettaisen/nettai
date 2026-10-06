--!strict
-- GreatYo (YoYo's Program Advance): YoYo's throw (chips/yoyo/throw.luau)
-- of a controller that sends three yoyos, one on each row, which roll out
-- four panels, spin for five rounds and move together.

local throw = require("./yoyo/throw")
local yoyo = require("./yoyo").yoyo

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    program_advances = { { order = 27, code_run = { chip = yoyo, count = 3 } } },
    action = throw.action { id = "greatyo/action", yoyo = { great = true, panels = 4, rounds = 5 } },
}

return { greatyo = chip }
