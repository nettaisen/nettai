--!strict
-- HeatDrgn: a fire dragon that dives down the first column ahead with an
-- enemy in it (lib/dragons).

local action = require("../../lib/dragons/action")
local dragon = require("../../lib/dragons/dragon")
local lockon = require("../../rules/lockon")
local sparks = require("../../lib/sparks")

return define.chip {
    id = "heatdrgn",
    name = "HeatDrgn",
    description = "Drgn near\nclst enmy\nHit 2rows",
    codes = { "G", "R", "T" },
    element = "fire", family = "fire", class = "standard",
    rarity = 2, mb = 40, damage = 140, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 123, index = 123, sort = 235 },
    slot_in_limit = 3,
    -- No Beast rush; its lock-on mode (rules/lockon).
    beast = { rush = false, lockon = lockon.cannon },
    action = action.action {
        dragon = dragon.variant {
            speed = 0x4_0000,
            palette = 0,
            hit_mod = 3,
            spark = sparks.fire,
            panel = false,
            delays = { 3, 6, 9, 12 },
        },
    },
}
