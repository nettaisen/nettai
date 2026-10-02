--!strict
-- AirShot: a wind shot from the panel ahead that pushes what it hits
-- (lib/airshot.luau: the original's action 0x21, `sub_80EC884`).

local airshot = require("../../lib/airshot")
local lockon = require("../../rules/lockon")
local variants = require("../../objects/projectile/variants")

return define.chip {
    id = "airshot",
    codes = { "*" },
    element = "null", family = "wind", class = "standard",
    rarity = 1, mb = 4, damage = 20, hit_param = 20, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { "free_slot_in" },
    library = { number = 4, index = 4, sort = 5 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.cannon },
    action = airshot.action { shooter = airshot.shooter, shot = variants.air },
}
