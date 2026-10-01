--!strict
-- ElcPuls1-3 and DestPuls: an electric pulse held out over the panel ahead
-- and the column past it, pulling what it hits (./action, ./pulse).

local action = require("./action")
local pulse = require("./pulse")
local lockon = require("../../rules/lockon")

-- Every pulse lasts 60 ticks; the navi holds it out 5 less, or 5 for the
-- original's second look (ElcPuls2's).
local TICKS = 0x3C
local HOLD, SHORT_HOLD = TICKS - 5, 5

local elcpuls1 = define.chip {
    id = "elcpuls1",
    name = "ElcPuls1",
    description = "Sprd elec\npuls that\nparalyzes",
    codes = { "J", "L", "S" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 32,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.elcpuls1 },
    library = { number = 33, index = 33, sort = 136 },
    slot_in_limit = 3,
    action = action.action {
        id = "elcpuls1/action",
        pulse = pulse.variant { hit_mod = 1, status = 0x10, ticks = TICKS, palette = 1 },
        hold = HOLD,
    },
}

local elcpuls2 = define.chip {
    id = "elcpuls2",
    name = "ElcPuls2",
    description = "Sprd elec\npulse.Pul\nenemy in.",
    codes = { "A", "E", "J" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 36,
    damage = 120,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.elcpuls1 },
    library = { number = 34, index = 34, sort = 137 },
    slot_in_limit = 3,
    action = action.action {
        id = "elcpuls2/action",
        pulse = pulse.variant { hit_mod = 0x51, status = 0, ticks = TICKS, palette = 2 },
        hold = SHORT_HOLD,
    },
}

local elcpuls3 = define.chip {
    id = "elcpuls3",
    name = "ElcPuls3",
    description = "Sprd elec\npuls. Hit\nw/HP bug.",
    codes = { "A", "J", "S" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 140,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.elcpuls1 },
    library = { number = 35, index = 35, sort = 138 },
    slot_in_limit = 3,
    action = action.action {
        id = "elcpuls3/action",
        -- Its hits carry bug 0x18 with argument 1.
        pulse = pulse.variant { hit_mod = 3, status = 0, ticks = TICKS, palette = 0, bug = 0x18, bug_arg = 1 },
        hold = HOLD,
    },
}

local destpuls = define.chip {
    id = "destpuls",
    name = "DestPuls",
    description = "Fire puls\nthat caus\nstrg bug!",
    codes = { "A", "B", "C", "*" },
    element = "elec",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    beast = { lockon = lockon.elcpuls1 },
    library = { number = 334, index = 15, sort = 0 },
    slot_in_limit = 1,
    program_advances = {
        {
            order = 4,
            sequence = { elcpuls1, elcpuls2, elcpuls3 },
        },
    },
    action = action.action {
        id = "destpuls/action",
        -- Its hits carry the HP bug 0xF6.
        pulse = pulse.variant { hit_mod = 0, status = 0x12, ticks = TICKS, palette = 3, bug = 0xF6 },
        hold = HOLD,
    },
}

return { elcpuls1 = elcpuls1, elcpuls2 = elcpuls2, elcpuls3 = elcpuls3, destpuls = destpuls }
