--!strict
-- IronShl1, IronShl2 and IronShl3: an iron shell thrown onto the panel
-- ahead that slides to the far column, bumps there once and flies off
-- (throw.luau); each faster (the original's by the chip's level,
-- `byte_80CA7A0`: 6, 7 and 8 pixels a tick) and in its own palette.

local throw = require("@self/throw")
local lockon = require("../rules/lockon")

local ironshl1: Chip = {
    codes = { "J", "K", "L" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 13,
    damage = 70,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.cannon },
    slot_in_limit = 3,
    action = throw.action { id = "ironshl1/action", shell = { palette = 0, speed = 0x6_0000, bumps = 1 } },
}

local ironshl2: Chip = {
    codes = { "C", "D", "E" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.cannon },
    slot_in_limit = 3,
    action = throw.action { id = "ironshl2/action", shell = { palette = 1, speed = 0x7_0000, bumps = 1 } },
}

local ironshl3: Chip = {
    codes = { "L", "M", "N" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 27,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.cannon },
    slot_in_limit = 3,
    action = throw.action { id = "ironshl3/action", shell = { palette = 2, speed = 0x8_0000, bumps = 1 } },
}

return { ironshl1 = ironshl1, ironshl2 = ironshl2, ironshl3 = ironshl3 }
