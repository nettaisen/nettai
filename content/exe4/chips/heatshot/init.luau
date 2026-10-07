--!strict
-- HeatShot, Heat-V and HeatSide (EXE4 chips 0x013 to 0x015; their records at 0x080197EC + n * 0x2C): the spreaders'
-- action 0x1E (./spreader/spreader), its variant 1: the HeatShots' gun (attachment row 6) fires past the tenth tick,
-- at height 0x10, bullet rows 8 to 10 (their parameter +0x10): a fire burst on the first thing it finds and the panel
-- behind it, the two panels diagonally behind it, or the panels above and below it.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bullet = require("../objects/bullet")
local spreader = require("./spreader/spreader")

local function fire(id: string, shot: BulletVariant): Action
    return spreader.action { id = id, gun = spreader.guns.heatshot, delay = 10, z = 0x10, shot = shot }
end

local heatshot: Chip = {
    action = fire("heatshot/action", bullet.variants.heat_long),
    codes = { "B", "C", "D", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 40,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local heat_v: Chip = {
    action = fire("heat-v/action", bullet.variants.heat_fork),
    codes = { "C", "D", "E" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 70,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local heatside: Chip = {
    action = fire("heatside/action", bullet.variants.heat_wide),
    codes = { "D", "E", "F" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 44,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { heatshot = heatshot, ["heat-v"] = heat_v, heatside = heatside }
