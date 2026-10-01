--!strict
-- DblDream: two big slashes on the two columns ahead.

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return define.chip {
    id = "dbldream",
    name = "DblDream",
    codes = { "E", "L", "S", "*" },
    element = "null", family = "sword", class = "special",
    rarity = 0, mb = 8, damage = 80, hit_param = 30, lockout = 0,
    flags = { "has_damage", "library" },
    library = { number = 376, index = 0, sort = 0 },
    slot_in_limit = 1,
    -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
    beast = { lockon = 6 },
    -- SlashCross's charged slash reads the chip's subtype from its record
    -- (transitional, until the charge asks the chip for its slash).
    legacy = { subtype = 11 },
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 1 },
        effect = parts.effects.big,
        sound = parts.sounds.big_swing,
        swings = 2,
    },
}
