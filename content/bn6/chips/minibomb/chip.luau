--!strict
-- MiniBomb: a bomb thrown three panels ahead that blasts the panel it
-- lands on.

local bomb = require("../../lib/bombs/bomb")
local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local throw = require("../../lib/bombs/throw")

return define.chip {
    id = "minibomb",
    name = "MiniBomb",
    codes = { "B", "L", "R", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 0, mb = 6, damage = 50, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 58, index = 58, sort = 317 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
    beast = { lockon = 5 },
    action = throw.action {
        held = bomb.held,
        thrower = bomb.thrower(bomb.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 0,
            palette = 0,
            anim = 1,
            blast = regions.single,
        }),
    },
}
