--!strict
-- TankCan1, TankCan2 and TankCan3: the tankcan series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local tankcan = require("./tankcan")

-- TankCan1 (BN5 chip 0x03f): a shell from the panel ahead that staggers what it hits, or, off the far edge, blasts
-- and cracks the last column and sets off three explosions there (chips/tankcan/tankcan: BN6's with BN5's look).
local tankcan1 = define.chip {
    id = "bn5:tankcan1",
    codes = { "A", "E", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 20,
    damage = 80,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = tankcan.action,
}

-- TankCan2 (BN5 chip 0x040): a shell from the panel ahead that staggers what it hits, or, off the far edge, blasts
-- and cracks the last column and sets off three explosions there (chips/tankcan/tankcan: BN6's with BN5's look).
local tankcan2 = define.chip {
    id = "bn5:tankcan2",
    codes = { "B", "T", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 120,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = tankcan.action,
}

-- TankCan3 (BN5 chip 0x041): a shell from the panel ahead that staggers what it hits, or, off the far edge, blasts
-- and cracks the last column and sets off three explosions there (chips/tankcan/tankcan: BN6's with BN5's look).
local tankcan3 = define.chip {
    id = "bn5:tankcan3",
    codes = { "F", "M", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 160,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = tankcan.action,
}

return { tankcan1 = tankcan1, tankcan2 = tankcan2, tankcan3 = tankcan3 }
