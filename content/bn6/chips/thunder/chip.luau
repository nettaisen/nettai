--!strict
-- Thunder: a thunder ball (ball.luau) that homes in on the enemy for 5
-- panels and paralyzes what it hits (shoot.luau).

local lockon = require("../../rules/lockon")
local shoot = require("./shoot")

-- Its hits' status: paralysis.
local PARALYZE = 0x10

return define.chip {
    id = "thunder",
    name = "Thunder",
    description = "Pralyzing\nelectric\nattack!",
    codes = { "B", "R", "S", "*" },
    element = "elec", family = "elec", class = "standard",
    rarity = 1, mb = 7, damage = 40, hit_param = 20, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 29, index = 29, sort = 433 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.thunder },
    action = shoot.action { ball = { panels = 5, status = PARALYZE } },
}
