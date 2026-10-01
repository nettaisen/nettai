--!strict
-- The second WhiCapsl (chip 0x17E, a special chip no folder holds): a
-- dimming chip that turns its user invisible for two seconds
-- (Invisibl's controller, chips/invisibl/controller).

local invisible = require("../invisibl/controller")

-- 120 ticks (the record's first two parameter bytes, 120 and 0).
return define.chip {
    id = "whicapsl-invisible",
    name = "WhiCapsl",
    description = "Add an\neffect:\nParalyze",
    codes = {},
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 30,
    damage = 0,
    hit_param = 50,
    lockout = 0,
    flags = { "dimming" },
    extra_flags = { "rush_cancels" },
    library = { number = 382, index = 0, sort = 0 },
    slot_in_limit = 1,
    traits = { "no_cut_in" },
    dimming = invisible.hook(120),
}
