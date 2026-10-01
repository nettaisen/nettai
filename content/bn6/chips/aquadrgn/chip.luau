--!strict
-- AquaDrgn: an aqua dragon that dives down the first column ahead with an
-- enemy in it, freezing the panels behind it (lib/dragons).

local action = require("../../lib/dragons/action")
local dragon = require("../../lib/dragons/dragon")
local sparks = require("../../lib/sparks")

return define.chip {
    id = "aquadrgn",
    name = "AquaDrgn",
    codes = { "H", "P", "S" },
    element = "aqua", family = "aqua", class = "standard",
    rarity = 3, mb = 44, damage = 120, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 125, index = 125, sort = 14 },
    slot_in_limit = 3,
    -- No Beast rush; its lock-on mode (a number until rules/lockon.luau).
    beast = { rush = false, lockon = 1 },
    action = action.action {
        dragon = dragon.variant {
            speed = 0x4_0000,
            palette = 6,
            hit_mod = 3,
            spark = sparks.aqua,
            panel = "ice",
            delays = { 3, 6, 9, 12 },
        },
    },
}
