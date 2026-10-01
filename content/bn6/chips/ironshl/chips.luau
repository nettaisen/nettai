--!strict
-- IronShl1, IronShl2 and IronShl3: an iron shell thrown onto the panel
-- ahead that slides to the far column, bumps there once and flies off
-- (throw.luau); each faster and in its own palette.

local throw = require("./throw")
local lockon = require("../../rules/lockon")

-- `byte_80CA7A0`: the shells' speeds, 16.16 a tick.
local SPEEDS = { 0x6_0000, 0x7_0000, 0x8_0000 }

local ironshl1 = define.chip {
    id = "ironshl1",
    name = "IronShl1",
    description = "Fire off\npiercing\nshell.",
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
    library = { number = 45, index = 45, sort = 245 },
    slot_in_limit = 3,
    action = throw.action { id = "ironshl1/action", shell = { palette = 0, speed = SPEEDS[1], bumps = 1 } },
}

local ironshl2 = define.chip {
    id = "ironshl2",
    name = "IronShl2",
    description = "Fire off\npiercing\nshell.",
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
    library = { number = 46, index = 46, sort = 246 },
    slot_in_limit = 3,
    action = throw.action { id = "ironshl2/action", shell = { palette = 1, speed = SPEEDS[2], bumps = 1 } },
}

local ironshl3 = define.chip {
    id = "ironshl3",
    name = "IronShl3",
    description = "Fire off\npiercing\nshell.",
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
    library = { number = 47, index = 47, sort = 247 },
    slot_in_limit = 3,
    action = throw.action { id = "ironshl3/action", shell = { palette = 2, speed = SPEEDS[3], bumps = 1 } },
}

return { ironshl1 = ironshl1, ironshl2 = ironshl2, ironshl3 = ironshl3 }
