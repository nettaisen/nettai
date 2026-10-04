--!strict
-- WoodDrgn: a wood dragon that dives down the first column ahead with an
-- enemy in it, growing grass behind it (lib/dragons).

local action = require("../../lib/dragons/action")
local dragon = require("../../lib/dragons/dragon")
local lockon = require("../../rules/lockon")
local sparks = require("../../lib/sparks")

return define.chip {
    id = "wooddrgn",
    codes = { "G", "T", "V" },
    element = "wood", family = "wood", class = "standard",
    rarity = 3, mb = 48, damage = 130, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 126, index = 126, sort = 512 },
    slot_in_limit = 3,
    -- No Beast rush; its lock-on mode (rules/lockon).
    beast = { rush = false, lockon = lockon.cannon },
    action = action.action {
        dragon = dragon.variant {
            speed = 0x4_0000,
            palette = 9,
            hit_mod = 3,
            spark = sparks.wood,
            panel = "grass",
            delays = { 3, 6, 9, 12 },
        },
    },
}
