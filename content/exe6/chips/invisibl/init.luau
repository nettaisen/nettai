--!strict
-- Invisibl: its user turns invisible for six seconds (its controller,
-- ./controller).

local invisible = require("@self/controller")

-- 360 ticks (the record's first two parameter bytes, 104 and 1).
return define.chip {
    id = "invisibl",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { "rush_cancels", "free_slot_in" },
    slot_in_limit = 3,
    dimming = invisible.hook(360),
}
