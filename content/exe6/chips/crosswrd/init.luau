--!strict
-- CrosSwrd: an X-shaped slash ahead, and a second hit on the panel ahead.

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

local chip: Chip = {
    codes = { "E", "L", "S", "*" },
    element = "null", family = "sword", class = "special",
    rarity = 0, mb = 8, damage = 80, hit_param = 30, lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.crosswrd },
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.x, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.cross,
        sound = parts.sounds.swing,
        second_hit = slash.CROSS_HIT,
        charged = slashes.cross,
    },
}

return { crosswrd = chip }
