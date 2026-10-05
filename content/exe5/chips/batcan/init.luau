--!strict
-- BatCan1, BatCan2, BatCan3 and BatCan4: the batcan series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local batcan = require("@self/batcan")

-- BatCan1 (EXE5 chip 0x13b): the navi raises its bat cannon and fires an unseen shot from the panel ahead on the 7th
-- tick: EXE6's BatCan (chips/batcan/batcan).
local batcan1 = define.chip {
    id = "batcan1",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage" },
    slot_in_limit = 1,
    action = batcan.action { shots = { 3 } },
}

-- BatCan2 (EXE5 chip 0x13c): the navi raises its bat cannon and fires an unseen shot from the panel ahead on the 7th
-- tick: EXE6's BatCan (chips/batcan/batcan).
local batcan2 = define.chip {
    id = "batcan2",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage" },
    slot_in_limit = 1,
    action = batcan.action { shots = { 3 } },
}

-- BatCan3 (EXE5 chip 0x13d): the navi raises its bat cannon and fires an unseen shot from the panel ahead on the 7th
-- tick: EXE6's BatCan (chips/batcan/batcan).
local batcan3 = define.chip {
    id = "batcan3",
    codes = { "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage" },
    slot_in_limit = 1,
    action = batcan.action { shots = { 3 } },
}

-- BatCan4 (EXE5 chip 0x13e): the navi raises its bat cannon and fires three unseen shots from the panel ahead, 16
-- ticks apart, the last staggering harder: EXE6's BatCan4 (chips/batcan/batcan).
local batcan4 = define.chip {
    id = "batcan4",
    codes = { "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage" },
    slot_in_limit = 1,
    action = batcan.action { shots = { 1, 1, 3 } },
}

return { batcan1 = batcan1, batcan2 = batcan2, batcan3 = batcan3, batcan4 = batcan4 }
