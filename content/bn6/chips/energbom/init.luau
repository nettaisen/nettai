--!strict
-- EnergBom and MegEnBom: a bomb that doesn't blast, but leaves an energy
-- burst where it lands (burst.luau): three hits on that panel.

local bomb = require("../lib/bombs/bomb")
local burst = require("@self/burst")
local collision = require("../rules/collision")
local lockon = require("../rules/lockon")
local throw = require("../lib/bombs/throw")

-- The two chips' throw: the same bomb, held in its second animation.
local function energy_throw(): Action
    return throw.action {
        held = bomb.held,
        held_anim = 2,
        thrower = bomb.thrower(bomb.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 1,
            palette = 0,
            anim = 3,
            after = burst.leave,
        }),
    }
end

return {
    define.chip {
        id = "energbom",
        codes = { "C", "K", "V", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 11, damage = 40, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 60, index = 60, sort = 149 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.yoyo },
        action = energy_throw(),
    },
    define.chip {
        id = "megenbom",
        codes = { "G", "M", "O", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 27, damage = 60, hit_param = 10, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 61, index = 61, sort = 313 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.yoyo },
        action = energy_throw(),
    },
}
