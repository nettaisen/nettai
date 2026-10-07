--!strict
-- Cannon, HiCannon and M-Cannon: a shot from the panel ahead that
-- staggers what it hits; the same cannon in three colors (the
-- original's action 0x14 with subtypes 0 to 2; lib/cannon.luau).

local attachment = require("../objects/attachment")
local shooter = require("../lib/cannon")
local variants = require("../objects/projectile/variants")
local lockon = require("../rules/lockon")

-- Their cannons (the original's attachment rows 0 to 2: the cannon in
-- palettes 0, 1 and 2), at the navi's attach point 7.
local CANNON_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 0, attach_point = 7 }
local HICANNON_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 1, attach_point = 7 }
local M_CANNON_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 2, attach_point = 7 }

local cannon: Chip = {
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
    slot_in_limit = 3,
    action = shooter.action { id = "cannon/action", arm = CANNON_ARM, shot = variants.cannon },
}

local hicannon: Chip = {
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
    slot_in_limit = 3,
    action = shooter.action { id = "hicannon/action", arm = HICANNON_ARM, shot = variants.cannon },
}

local m_cannon: Chip = {
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
    slot_in_limit = 3,
    action = shooter.action { id = "m-cannon/action", arm = M_CANNON_ARM, shot = variants.cannon },
}

return { cannon = cannon, hicannon = hicannon, ["m-cannon"] = m_cannon }
