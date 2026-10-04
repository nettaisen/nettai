--!strict
-- CannMode, CannBall, SwrdMode, YoYoMode and DrilMode: the cannmode series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local side_special = require("@exelib/instant/side_special")

-- CannMode (BN5 chip 0x119): one of the Liberation Missions' mode chips: instant chip effect 11 (0x0802D62A, BN6's
-- `sub_802E1BE`, @exelib/instant/side_special; the same code) writes its mode (the record's first parameter) and a
-- 480-tick timer (360 in the operation battle) to the user's side state, which nothing in a battle reads (0x0800E576
-- counts the timer down), so the chip's use changes nothing observable.
local cannmode = define.chip {
    id = "cannmode",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 10,
    damage = 40,
    hit_param = 20,
    lockout = 20,
    flags = {},
    extra_flags = { 32 },
    slot_in_limit = 1,
    instant = side_special.instant,
}

-- CannBall (BN5 chip 0x11a): one of the Liberation Missions' mode chips: instant chip effect 11 (0x0802D62A, BN6's
-- `sub_802E1BE`, @exelib/instant/side_special; the same code) writes its mode (the record's first parameter) and a
-- 480-tick timer (360 in the operation battle) to the user's side state, which nothing in a battle reads (0x0800E576
-- counts the timer down), so the chip's use changes nothing observable.
local cannball_mode = define.chip {
    id = "cannball-mode",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 30,
    damage = 130,
    hit_param = 30,
    lockout = 20,
    flags = {},
    extra_flags = { 32 },
    slot_in_limit = 1,
    instant = side_special.instant,
}

-- SwrdMode (BN5 chip 0x11b): one of the Liberation Missions' mode chips: instant chip effect 11 (0x0802D62A, BN6's
-- `sub_802E1BE`, @exelib/instant/side_special; the same code) writes its mode (the record's first parameter) and a
-- 480-tick timer (360 in the operation battle) to the user's side state, which nothing in a battle reads (0x0800E576
-- counts the timer down), so the chip's use changes nothing observable.
local swrdmode = define.chip {
    id = "swrdmode",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 40,
    damage = 80,
    hit_param = 30,
    lockout = 20,
    flags = {},
    extra_flags = { 32 },
    slot_in_limit = 1,
    instant = side_special.instant,
}

-- YoYoMode (BN5 chip 0x11c): one of the Liberation Missions' mode chips: instant chip effect 11 (0x0802D62A, BN6's
-- `sub_802E1BE`, @exelib/instant/side_special; the same code) writes its mode (the record's first parameter) and a
-- 480-tick timer (360 in the operation battle) to the user's side state, which nothing in a battle reads (0x0800E576
-- counts the timer down), so the chip's use changes nothing observable.
local yoyomode = define.chip {
    id = "yoyomode",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 50,
    damage = 50,
    hit_param = 10,
    lockout = 20,
    flags = {},
    extra_flags = { 32 },
    slot_in_limit = 1,
    instant = side_special.instant,
}

-- DrilMode (BN5 chip 0x11d): one of the Liberation Missions' mode chips: instant chip effect 11 (0x0802D62A, BN6's
-- `sub_802E1BE`, @exelib/instant/side_special; the same code) writes its mode (the record's first parameter) and a
-- 480-tick timer (360 in the operation battle) to the user's side state, which nothing in a battle reads (0x0800E576
-- counts the timer down), so the chip's use changes nothing observable.
local drilmode = define.chip {
    id = "drilmode",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 80,
    damage = 60,
    hit_param = 10,
    lockout = 20,
    flags = {},
    slot_in_limit = 1,
    instant = side_special.instant,
}

return {
    cannmode = cannmode,
    cannball_mode = cannball_mode,
    swrdmode = swrdmode,
    yoyomode = yoyomode,
    drilmode = drilmode,
}
