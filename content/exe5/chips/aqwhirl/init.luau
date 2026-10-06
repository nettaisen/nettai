--!strict
-- AqWhirl1, AqWhirl2 and AqWhirl3: the aqwhirl series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local action = require("@exelib/airspin/action")
local whirlpool = require("@self/whirlpool")

-- AqWhirl1 (EXE5 chip 0x09d): action 0x47 as variant 0 (@exelib/airspin/action): a whirlpool
-- (./whirlpool) set down ahead in palette 0 at speed 0x20, looping round the field.
local aqwhirl1: Chip = {
    codes = { "B", "F", "N" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 30,
    damage = 70,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = action.action { whirlwind = whirlpool.sender(0, 0x20) },
}

-- AqWhirl2 (EXE5 chip 0x09e): action 0x47 as variant 0 (@exelib/airspin/action): a whirlpool
-- (./whirlpool) set down ahead in palette 1 at speed 0x30, looping round the field.
local aqwhirl2: Chip = {
    codes = { "A", "H", "O" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 42,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = action.action { whirlwind = whirlpool.sender(1, 0x30) },
}

-- AqWhirl3 (EXE5 chip 0x09f): action 0x47 as variant 0 (@exelib/airspin/action): a whirlpool
-- (./whirlpool) set down ahead in palette 2 at speed 0x50, looping round the field.
local aqwhirl3: Chip = {
    codes = { "D", "I", "R" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 54,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = action.action { whirlwind = whirlpool.sender(2, 0x50) },
}

return { aqwhirl1 = aqwhirl1, aqwhirl2 = aqwhirl2, aqwhirl3 = aqwhirl3 }
