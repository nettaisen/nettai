--!strict
-- MrkCan1, MrkCan2 and MrkCan3: the mrkcan series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local mrkcan = require("@self/mrkcan")

-- MrkCan1 (EXE5 chip 0x00c): a sight sweeps the field ahead and locks on to the first enemy it finds, paralyzing it,
-- and the cannon fires at it (EXE5's own MarkCannon, chips/mrkcan/mrkcan: action 0x4E).
local mrkcan1 = define.chip {
    id = "mrkcan1",
    codes = { "M", "K", "S" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 0,
    mb = 18,
    damage = 70,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = mrkcan.action { cannon = mrkcan.cannon(0) },
}

-- MrkCan2 (EXE5 chip 0x00d): a sight sweeps the field ahead and locks on to the first enemy it finds, paralyzing it,
-- and the cannon fires at it (EXE5's own MarkCannon, chips/mrkcan/mrkcan: action 0x4E).
local mrkcan2 = define.chip {
    id = "mrkcan2",
    codes = { "D", "F", "Z" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 1,
    mb = 27,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = mrkcan.action { cannon = mrkcan.cannon(3) },
}

-- MrkCan3 (EXE5 chip 0x00e): a sight sweeps the field ahead and locks on to the first enemy it finds, paralyzing it,
-- and the cannon fires at it (EXE5's own MarkCannon, chips/mrkcan/mrkcan: action 0x4E).
local mrkcan3 = define.chip {
    id = "mrkcan3",
    codes = { "E", "R", "Y" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 2,
    mb = 36,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = mrkcan.action { cannon = mrkcan.cannon(6) },
}

return { mrkcan1 = mrkcan1, mrkcan2 = mrkcan2, mrkcan3 = mrkcan3 }
