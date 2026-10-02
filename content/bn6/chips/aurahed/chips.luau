--!strict
-- AuraHed1-3 and StreamHd: an aura head fired along the row (./action,
-- ./head); StreamHd fires five.

local action = require("./action")
local lockon = require("../../rules/lockon")

-- Every head charges at 12 pixels a tick (`byte_80D74D4`).
local SPEED = 0xC_0000

local aurahed1 = define.chip {
    id = "aurahed1",
    codes = { "B", "C", "D" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 25,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.yoyo },
    library = { number = 48, index = 48, sort = 23 },
    slot_in_limit = 3,
    traits = { "aura_bonus" },
    action = action.action {
        id = "aurahed1/action",
        head = { palette = 0, speed = SPEED, strong = true, far = false },
    },
}

local aurahed2 = define.chip {
    id = "aurahed2",
    codes = { "D", "E", "F" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 1,
    mb = 33,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.yoyo },
    library = { number = 49, index = 49, sort = 24 },
    slot_in_limit = 3,
    traits = { "aura_bonus" },
    action = action.action {
        id = "aurahed2/action",
        head = { palette = 1, speed = SPEED, strong = true, far = false },
    },
}

local aurahed3 = define.chip {
    id = "aurahed3",
    codes = { "F", "G", "H" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 39,
    damage = 170,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.yoyo },
    library = { number = 50, index = 50, sort = 25 },
    slot_in_limit = 3,
    traits = { "aura_bonus" },
    action = action.action {
        id = "aurahed3/action",
        head = { palette = 2, speed = SPEED, strong = true, far = false },
    },
}

local streamhd = define.chip {
    id = "streamhd",
    codes = {},
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 150,
    hit_param = 5,
    lockout = 20,
    flags = { "has_damage", "library" },
    beast = { lockon = lockon.cannon },
    library = { number = 336, index = 17, sort = 0 },
    slot_in_limit = 1,
    program_advances = {
        { order = 40, code_run = { chip = aurahed1, count = 3 } },
        { order = 41, code_run = { chip = aurahed2, count = 3 } },
        { order = 42, code_run = { chip = aurahed3, count = 3 } },
    },
    traits = { "aura_bonus" },
    -- Five heads to the far edge, the last one strong.
    action = action.action {
        id = "streamhd/action",
        head = { palette = 0, speed = SPEED, strong = false, far = true },
        stream = true,
    },
}

return { aurahed1 = aurahed1, aurahed2 = aurahed2, aurahed3 = aurahed3, streamhd = streamhd }
