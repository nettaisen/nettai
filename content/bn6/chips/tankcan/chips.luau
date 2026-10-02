--!strict
-- TankCan1-3: a shell from the panel ahead (action.luau, shell.luau) that
-- staggers what it hits, or, off the far edge, blasts and cracks the last
-- column and sets off three explosions there. The three differ only in
-- their damage.

local lockon = require("../../rules/lockon")
local tankcan = require("./action")

-- The shell: kind 0's hit modifier, cracking, no damage shift (the chips'
-- parameters 0x100).
local SHELL = { hit_mod = 0x4B, cracks = true, shift = 0 }

return {
    define.chip {
        id = "tankcan1",
        codes = { "A", "G", "R" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 17, damage = 120, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 12, index = 12, sort = 427 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = tankcan.action { shell = SHELL },
    },
    define.chip {
        id = "tankcan2",
        codes = { "L", "S", "V" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 28, damage = 160, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 13, index = 13, sort = 428 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = tankcan.action { shell = SHELL },
    },
    define.chip {
        id = "tankcan3",
        codes = { "B", "M", "P" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 39, damage = 200, hit_param = 20, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 14, index = 14, sort = 429 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.cannon },
        action = tankcan.action { shell = SHELL },
    },
}
