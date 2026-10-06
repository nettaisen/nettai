--!strict
-- ElecDrgn: an elec dragon that dives down the first column ahead with an
-- enemy in it, cracking the panels behind it (lib/dragons).

local action = require("../lib/dragons/action")
local dragon = require("../lib/dragons/dragon")
local lockon = require("../rules/lockon")
local sparks = require("../lib/sparks")

local chip: Chip = {
    codes = { "A", "L", "V" },
    element = "elec", family = "elec", class = "standard",
    rarity = 2, mb = 40, damage = 150, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    -- No Beast rush; its lock-on mode (rules/lockon).
    beast = { rush = false, lockon = lockon.cannon },
    action = action.action {
        dragon = dragon.variant {
            speed = 0x4_0000,
            palette = 3,
            hit_mod = 3,
            spark = sparks.elec,
            panel = "cracked",
            delays = { 3, 6, 9, 12 },
        },
    },
}

return { elecdrgn = chip }
