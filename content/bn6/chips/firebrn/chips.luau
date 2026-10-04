--!strict
-- FireBrn1, FireBrn2 and FireBrn3: the burner's flame (lib/burner) on the
-- panel ahead, burning 30 ticks and spreading two panels forward, cracking
-- the panels it leaves. The three differ only in their damage.

local burn = require("../../lib/burner/burn")
local lockon = require("../../rules/lockon")

local function fire_burn(id: string): Action
    return burn.action {
        id = id,
        burner = burn.burners.fire,
        flame = { ticks = 30, spread = 2, cracks = true },
    }
end

local firebrn1 = define.chip {
    id = "firebrn1",
    codes = { "F", "G", "H", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 70,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.firebrn1 },
    library = { number = 19, index = 19, sort = 203 },
    slot_in_limit = 3,
    action = fire_burn("firebrn1/action"),
}

local firebrn2 = define.chip {
    id = "firebrn2",
    codes = { "S", "T", "U" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 21,
    damage = 110,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.firebrn1 },
    library = { number = 20, index = 20, sort = 204 },
    slot_in_limit = 3,
    action = fire_burn("firebrn2/action"),
}

local firebrn3 = define.chip {
    id = "firebrn3",
    codes = { "C", "D", "E" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 34,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.firebrn1 },
    library = { number = 21, index = 21, sort = 205 },
    slot_in_limit = 3,
    action = fire_burn("firebrn3/action"),
}

return { firebrn1 = firebrn1, firebrn2 = firebrn2, firebrn3 = firebrn3 }
