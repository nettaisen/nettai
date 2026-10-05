--!strict
-- Recov10, Recov30, Recov50, Recov80, Recov120, Recov150, Recov200 and Recov300: the recov series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local heal = require("@exelib/recov/heal")

-- Recov10 (EXE5 chip 0x06b): the navi heals 10 HP in the tick after it uses it: EXE6's heal (@exelib/recov/heal,
-- the original's action 0x17, EXE6's 0x20; EXE5's 0x080EC484 the same heal and table, 0x080EC4B8) and, unless an
-- AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (30).
local recov10 = define.chip {
    id = "recov10",
    codes = { "A", "C", "L", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 0,
    hit_param = 30,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
    action = heal.action { hp = 10, mood = 30 },
}

-- Recov30 (EXE5 chip 0x06c): the navi heals 30 HP in the tick after it uses it: EXE6's heal (@exelib/recov/heal,
-- the original's action 0x17, EXE6's 0x20; EXE5's 0x080EC484 the same heal and table, 0x080EC4B8) and, unless an
-- AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (30).
local recov30 = define.chip {
    id = "recov30",
    codes = { "C", "H", "L", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 16,
    damage = 0,
    hit_param = 30,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
    action = heal.action { hp = 30, mood = 30 },
}

-- Recov50 (EXE5 chip 0x06d): the navi heals 50 HP in the tick after it uses it: EXE6's heal (@exelib/recov/heal,
-- the original's action 0x17, EXE6's 0x20; EXE5's 0x080EC484 the same heal and table, 0x080EC4B8) and, unless an
-- AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (40).
local recov50 = define.chip {
    id = "recov50",
    codes = { "B", "H", "O", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 24,
    damage = 0,
    hit_param = 40,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
    action = heal.action { hp = 50, mood = 40 },
}

-- Recov80 (EXE5 chip 0x06e): the navi heals 80 HP in the tick after it uses it: EXE6's heal (@exelib/recov/heal,
-- the original's action 0x17, EXE6's 0x20; EXE5's 0x080EC484 the same heal and table, 0x080EC4B8) and, unless an
-- AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (40).
local recov80 = define.chip {
    id = "recov80",
    codes = { "B", "K", "O", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 1,
    mb = 32,
    damage = 0,
    hit_param = 40,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
    action = heal.action { hp = 80, mood = 40 },
}

-- Recov120 (EXE5 chip 0x06f): the navi heals 120 HP in the tick after it uses it: EXE6's heal (@exelib/recov/heal,
-- the original's action 0x17, EXE6's 0x20; EXE5's 0x080EC484 the same heal and table, 0x080EC4B8) and, unless an
-- AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (50).
local recov120 = define.chip {
    id = "recov120",
    codes = { "K" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
    action = heal.action { hp = 120, mood = 50 },
}

-- Recov150 (EXE5 chip 0x070): the navi heals 150 HP in the tick after it uses it: EXE6's heal (@exelib/recov/heal,
-- the original's action 0x17, EXE6's 0x20; EXE5's 0x080EC484 the same heal and table, 0x080EC4B8) and, unless an
-- AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (50).
local recov150 = define.chip {
    id = "recov150",
    codes = { "Q" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 48,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
    action = heal.action { hp = 150, mood = 50 },
}

-- Recov200 (EXE5 chip 0x071): the navi heals 200 HP in the tick after it uses it: EXE6's heal (@exelib/recov/heal,
-- the original's action 0x17, EXE6's 0x20; EXE5's 0x080EC484 the same heal and table, 0x080EC4B8) and, unless an
-- AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (50).
local recov200 = define.chip {
    id = "recov200",
    codes = { "I" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 56,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
    action = heal.action { hp = 200, mood = 50 },
}

-- Recov300 (EXE5 chip 0x072): the navi heals 300 HP in the tick after it uses it: EXE6's heal (@exelib/recov/heal,
-- the original's action 0x17, EXE6's 0x20; EXE5's 0x080EC484 the same heal and table, 0x080EC4B8) and, unless an
-- AntiRecv turned it, the side's mood gains the use's +0x0A, the record's parameter (50).
local recov300 = define.chip {
    id = "recov300",
    codes = { "Y" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 3,
    mb = 64,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
    action = heal.action { hp = 300, mood = 50 },
}

return {
    recov10 = recov10,
    recov30 = recov30,
    recov50 = recov50,
    recov80 = recov80,
    recov120 = recov120,
    recov150 = recov150,
    recov200 = recov200,
    recov300 = recov300,
}
