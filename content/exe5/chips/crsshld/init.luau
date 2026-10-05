--!strict
-- CrsShld1, CrsShld2 and CrsShld3: the crsshld series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local guard = require("../lib/guard")

-- CrsShld1 (EXE5 chip 0x0a9): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- cross along the row that strikes the first navi it meets: EXE5's guard (lib/guard, action 0x1F with subtype 5).
local crsshld1 = define.chip {
    id = "crsshld1",
    codes = { "B", "F", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 28,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = guard.looks.crsshld1, counter = { kind = "cross", palette = 0 } },
}

-- CrsShld2 (EXE5 chip 0x0aa): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- cross along the row that strikes the first navi it meets: EXE5's guard (lib/guard, action 0x1F with subtype 6).
local crsshld2 = define.chip {
    id = "crsshld2",
    codes = { "C", "M", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 160,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = guard.looks.crsshld2, counter = { kind = "cross", palette = 1 } },
}

-- CrsShld3 (EXE5 chip 0x0ab): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- cross along the row that strikes the first navi it meets: EXE5's guard (lib/guard, action 0x1F with subtype 7).
local crsshld3 = define.chip {
    id = "crsshld3",
    codes = { "D", "O", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 52,
    damage = 190,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = guard.looks.crsshld3, counter = { kind = "cross", palette = 2 } },
}

return { crsshld1 = crsshld1, crsshld2 = crsshld2, crsshld3 = crsshld3 }
