--!strict
-- WideBrn1, WideBrn2 and WideBrn3 (FireBrn's Program Advances): the
-- burner's flame (lib/burner) on the panel ahead, burning 60 ticks and
-- spreading two panels forward on three rows, cracking the panels it
-- leaves. The three differ only in their damage.

local attachment = require("../objects/attachment")
local burn = require("../lib/burner/burn")
local firebrn_chips = require("./firebrn")
local lockon = require("../rules/lockon")

local function wide_burn(id: string): Action
    return burn.action {
        id = id,
        -- The burner (the original's attachment row 0x16).
        burner = attachment.look { sprite = asset.sprite("burner"), palette = 0, attach_point = 0x14 },
        flame = { ticks = 60, spread = 2, wide = true, cracks = true },
    }
end

local widebrn1: Chip = {
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
    slot_in_limit = 1,
    program_advances = { { order = 17, code_run = { chip = firebrn_chips.firebrn1, count = 3 } } },
    action = wide_burn("widebrn1/action"),
}

local widebrn2: Chip = {
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
    slot_in_limit = 1,
    program_advances = { { order = 18, code_run = { chip = firebrn_chips.firebrn2, count = 3 } } },
    action = wide_burn("widebrn2/action"),
}

local widebrn3: Chip = {
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
    slot_in_limit = 1,
    program_advances = { { order = 19, code_run = { chip = firebrn_chips.firebrn3, count = 3 } } },
    action = wide_burn("widebrn3/action"),
}

return { widebrn1 = widebrn1, widebrn2 = widebrn2, widebrn3 = widebrn3 }
