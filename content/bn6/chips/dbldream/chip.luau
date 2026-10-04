--!strict
-- DblDream: two big slashes on the two columns ahead.

local lockon = require("../../rules/lockon")
local parts = require("../../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")

return define.chip {
    id = "bn6:dbldream",
    codes = { "E", "L", "S", "*" },
    element = "null", family = "sword", class = "special",
    rarity = 0, mb = 8, damage = 80, hit_param = 30, lockout = 0,
    flags = { "has_damage", "library" },
    library = { number = 376, index = 0, sort = 0 },
    slot_in_limit = 1,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.drksword },
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 1 },
        effect = parts.effects.big,
        sound = parts.sounds.big_swing,
        swings = 2,
        charged = slashes.double_dream,
    },
}
