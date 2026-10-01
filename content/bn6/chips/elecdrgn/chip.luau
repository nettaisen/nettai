--!strict
-- ElecDrgn: an elec dragon that dives down the first column ahead with an
-- enemy in it, cracking the panels behind it (lib/dragons).

local action = require("../../lib/dragons/action")
local dragon = require("../../lib/dragons/dragon")
local sparks = require("../../lib/sparks")

return define.chip {
    id = "elecdrgn",
    name = "ElecDrgn",
    description = "Drgn near\nclst enmy\nHit 2rows",
    codes = { "A", "L", "V" },
    element = "elec", family = "elec", class = "standard",
    rarity = 2, mb = 40, damage = 150, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 124, index = 124, sort = 139 },
    slot_in_limit = 3,
    -- No Beast rush; its lock-on mode (a number until rules/lockon.luau).
    beast = { rush = false, lockon = 1 },
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
