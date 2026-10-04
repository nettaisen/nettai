--!strict
-- H-Burst: a shot along the row that bursts ten times where it hits
-- (./action).

local action = require("./action")
local lockon = require("../../rules/lockon")
local spreadr = require("../spreadr/chips")

return define.chip {
    id = "h-burst",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 60,
    hit_param = 5,
    lockout = 0,
    flags = { "has_damage", "library" },
    beast = { lockon = lockon.cannon },
    library = { number = 338, index = 19, sort = 0 },
    slot_in_limit = 1,
    program_advances = {
        { order = 23, code_run = { chip = spreadr.spreadr1, count = 3 } },
        { order = 24, code_run = { chip = spreadr.spreadr2, count = 3 } },
        { order = 25, code_run = { chip = spreadr.spreadr3, count = 3 } },
    },
    -- Its first parameter: ten bursts.
    action = action.action { id = "h-burst/action", bursts = 10 },
}
