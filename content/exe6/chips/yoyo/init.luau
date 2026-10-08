--!strict
-- YoYo: a yoyo thrown from the panel ahead that rolls out two panels,
-- spins there for two rounds and rolls back (throw.luau).

local lockon = require("../rules/lockon")
local throw = require("@self/throw")
local yoyo = require("@self/yoyo")

local chip: Chip = {
    codes = { "L", "M", "N", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 50,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.yoyo },
    slot_in_limit = 3,
    action = throw.action { id = "yoyo/action", throw = yoyo.one { panels = 2, rounds = 2 } },
}

return { yoyo = chip }
