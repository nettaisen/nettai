--!strict
-- Bubbler, Bub-V and BublSide (EXE4 chips 0x016 to 0x018; their records at 0x080197EC + n * 0x2C): the spreaders'
-- action 0x1E (./spreader/spreader), its variant 2: the Bubblers' gun (attachment row 0x0E) fires past the tenth
-- tick, from the ground (height 0), bullet rows 4 to 6 (their parameter +0x10): a bubble burst on the first thing it
-- finds and the panel behind it, the two panels diagonally behind it, or the panels above and below it.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bullet = require("../objects/bullet")
local spreader = require("./spreader/spreader")

local function fire(id: string, shot: BulletVariant): Action
    return spreader.action { id = id, gun = spreader.guns.bubbler, delay = 10, z = 0, shot = shot }
end

local bubbler: Chip = {
    action = fire("bubbler/action", bullet.variants.bubble_long),
    codes = { "P", "Q", "R", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 12,
    damage = 50,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local bub_v: Chip = {
    action = fire("bub-v/action", bullet.variants.bubble_fork),
    codes = { "C", "D", "E" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local bublside: Chip = {
    action = fire("bublside/action", bullet.variants.bubble_wide),
    codes = { "D", "E", "F" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 44,
    damage = 110,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { bubbler = bubbler, ["bub-v"] = bub_v, bublside = bublside }
