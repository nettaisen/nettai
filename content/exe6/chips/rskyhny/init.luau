--!strict
-- RskyHny1-3: a hive that sends a bee ahead, and another for each hit the
-- navi takes in the next three windows (./action, ./bee); each level's
-- bees fly faster.

local action = require("@self/action")
local lockon = require("../rules/lockon")


-- Their bees (the original's by the chips' first parameter, their level,
-- `byte_80D3164`): the palette, and the speeds across and along the field.
local BEE1: HoneyBee = { palette = 0, speed = 0x2_8000, turn_speed = 0x1_8000 }
local BEE2: HoneyBee = { palette = 1, speed = 0x3_0000, turn_speed = 0x1_CCCC }
local BEE3: HoneyBee = { palette = 2, speed = 0x3_8000, turn_speed = 0x2_1999 }

local chips: { [string]: Chip } = {
    rskyhny1 = {
        codes = { "B", "G", "S" },
        element = "wood", family = "wood", class = "standard",
        rarity = 0, mb = 21, damage = 10, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { bee = BEE1 },
    },
    rskyhny2 = {
        codes = { "C", "R", "V" },
        element = "wood", family = "wood", class = "standard",
        rarity = 1, mb = 28, damage = 15, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = action.action { bee = BEE2 },
    },
    rskyhny3 = {
        codes = { "A", "D", "M" },
        element = "wood",
        family = "wood",
        class = "standard",
        rarity = 2,
        mb = 35,
        damage = 20,
        hit_param = 30,
        lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        beast = { lockon = lockon.cannon },
        slot_in_limit = 3,
        action = action.action { id = "rskyhny3/action", bee = BEE3 },
    },
}

return chips
