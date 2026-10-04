--!strict
-- SandWrm1, SandWrm2, SandWrm3: behind each of the other side's navis, a
-- worm (worm.luau) bursts out of the sand and arcs over it toward the
-- user; its level, the chip's (0 to 2), is its palette. (Their records'
-- lock-on mode is 1, without the Beast rush.)

local lockon = require("../rules/lockon")
local worm = require("@self/worm")

return {
    sandwrm1 = define.chip {
        id = "sandwrm1",
        codes = { "A", "G", "L" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 30, damage = 130, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        beast = { rush = false, lockon = lockon.cannon },
        library = { number = 97, index = 97, sort = 401 },
        slot_in_limit = 3,
        instant = worm.instant(0),
    },
    sandwrm2 = define.chip {
        id = "sandwrm2",
        codes = { "B", "R", "Y" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 34, damage = 150, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        beast = { rush = false, lockon = lockon.cannon },
        library = { number = 98, index = 98, sort = 402 },
        slot_in_limit = 3,
        instant = worm.instant(1),
    },
    sandwrm3 = define.chip {
        id = "sandwrm3",
        codes = { "H", "J", "S" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 38, damage = 170, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        beast = { rush = false, lockon = lockon.cannon },
        library = { number = 99, index = 99, sort = 403 },
        slot_in_limit = 3,
        instant = worm.instant(2),
    },
}
