--!strict
-- Cannon, HiCannon and M-Cannon: a shot from the panel ahead that
-- staggers what it hits; the same cannon in three colours (the
-- original's action 0x14 with subtypes 0 to 2; lib/cannon.luau).

local shooter = require("../../lib/cannon")
local variants = require("../../objects/projectile/variants")
local lockon = require("../../rules/lockon")

local cannon = define.chip {
    id = "cannon",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.cannon },
    library = { number = 1, index = 1, sort = 52 },
    slot_in_limit = 3,
    action = shooter.action { id = "cannon/action", arm = shooter.arms.cannon, shot = variants.cannon },
}

local hicannon = define.chip {
    id = "hicannon",
    codes = { "L", "M", "N", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 24,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.cannon },
    library = { number = 2, index = 2, sort = 240 },
    slot_in_limit = 3,
    action = shooter.action { id = "hicannon/action", arm = shooter.arms.hicannon, shot = variants.cannon },
}

local m_cannon = define.chip {
    id = "m-cannon",
    codes = { "R", "S", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 38,
    damage = 180,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.cannon },
    library = { number = 3, index = 3, sort = 311 },
    slot_in_limit = 3,
    action = shooter.action { id = "m-cannon/action", arm = shooter.arms.m_cannon, shot = variants.cannon },
}

return { cannon = cannon, hicannon = hicannon, m_cannon = m_cannon }
