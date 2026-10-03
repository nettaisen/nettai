--!strict
-- BigBomb: MiniBomb's throw with a bigger blast, the 3x3 around the panel
-- it lands on.

local bomb = require("../../lib/bombs/bomb")
local collision = require("../../rules/collision")
local lockon = require("../../rules/lockon")
local regions = require("@common/regions")
local throw = require("../../lib/bombs/throw")

return define.chip {
    id = "bn6:bigbomb",
    codes = { "O", "P", "V" },
    element = "null", family = "null", class = "standard",
    rarity = 3, mb = 32, damage = 140, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 59, index = 59, sort = 37 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.bigbomb },
    action = throw.action {
        held = bomb.held,
        held_palette = 3,
        thrower = bomb.thrower(bomb.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 3,
            palette = 3,
            anim = 1,
            blast = regions.square,
        }),
    },
}
