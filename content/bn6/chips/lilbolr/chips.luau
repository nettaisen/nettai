--!strict
-- LilBolr1-3: a boiler (boiler.luau) lobbed three panels ahead, that grows
-- its damage with every hit it takes and erupts over the eight panels
-- around it (the original's action 0x12 with subtype 3). The three differ
-- in their damage and level (the boiler's palette and the damage its body
-- deals: the original's first parameter, 0, 2, 3).

local bomb = require("../../lib/bombs/bomb")
local boiler = require("./boiler")
local throw = require("../../lib/bombs/throw")

local function boiler_throw(level: number): Action
    return throw.action { held = bomb.held, thrower = boiler.thrower(level) }
end

return {
    define.chip {
        id = "bn6:lilbolr1",
        codes = { "F", "K", "L" },
        element = "null", family = "summon", class = "standard",
        rarity = 0, mb = 18, damage = 100, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 139, index = 139, sort = 300 },
        slot_in_limit = 3,
        action = boiler_throw(0),
    },
    define.chip {
        id = "bn6:lilbolr2",
        codes = { "E", "M", "V" },
        element = "null", family = "summon", class = "standard",
        rarity = 1, mb = 23, damage = 140, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 140, index = 140, sort = 301 },
        slot_in_limit = 3,
        action = boiler_throw(2),
    },
    define.chip {
        id = "bn6:lilbolr3",
        codes = { "G", "S", "Z" },
        element = "null", family = "summon", class = "standard",
        rarity = 2, mb = 28, damage = 180, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 141, index = 141, sort = 302 },
        slot_in_limit = 3,
        action = boiler_throw(3),
    },
}
