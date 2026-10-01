--!strict
-- WideBrn1, WideBrn2 and WideBrn3 (FireBrn's Program Advances): the
-- burner's flame (lib/burner) on the panel ahead, burning 60 ticks and
-- spreading two panels forward on three rows, cracking the panels it
-- leaves. The three differ only in their damage.

local burn = require("../../lib/burner/burn")
local firebrn = require("../firebrn/chips")
local lockon = require("../../rules/lockon")

local function wide_burn(id: string): Action
    return burn.action {
        id = id,
        burner = burn.burners.fire,
        flame = { ticks = 60, spread = 2, wide = true, cracks = true },
    }
end

local widebrn1 = define.chip {
    id = "widebrn1",
    name = "WideBrn1",
    description = "An expnd\n3 row \nfire attk",
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 300,
    hit_param = 2,
    lockout = 0,
    flags = { "has_damage", "library" },
    beast = { lockon = lockon.gundels1 },
    library = { number = 323, index = 4, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 17, code_run = { chip = firebrn.firebrn1, count = 3 } } },
    action = wide_burn("widebrn1/action"),
}

local widebrn2 = define.chip {
    id = "widebrn2",
    name = "WideBrn2",
    description = "An expnd\n3 row \nfire attk",
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 350,
    hit_param = 2,
    lockout = 0,
    flags = { "has_damage", "library" },
    beast = { lockon = lockon.gundels1 },
    library = { number = 324, index = 5, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 18, code_run = { chip = firebrn.firebrn2, count = 3 } } },
    action = wide_burn("widebrn2/action"),
}

local widebrn3 = define.chip {
    id = "widebrn3",
    name = "WideBrn3",
    description = "An expnd\n3 row \nfire attk",
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 2,
    lockout = 0,
    flags = { "has_damage", "library" },
    beast = { lockon = lockon.gundels1 },
    library = { number = 325, index = 6, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 19, code_run = { chip = firebrn.firebrn3, count = 3 } } },
    action = wide_burn("widebrn3/action"),
}

return { widebrn1 = widebrn1, widebrn2 = widebrn2, widebrn3 = widebrn3 }
