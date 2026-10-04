--!strict
-- TimeBom1-3 and TimeBom+ (the Program Advance of three TimeBoms of one
-- level in a run of codes): a bomb on the first free enemy panel ahead
-- that counts 3, 2, 1 and blows up the whole enemy area unless it is
-- broken first (./controller, ./countdown). TimeBom+'s has four times the
-- HP, its own look, and shakes the camera harder.

local collision = require("../../rules/collision")
local controller = require("./controller")
local countdown = require("./countdown")

-- What both bombs are and what they react to: a neutral body, hit by
-- everything; the one field object of their side's own class.
local BODY = { is = collision.neutral_body, hits = collision.everything }

-- The bombs (the original's rows 0 and 1): the animations they rise and
-- stand with, their HP and their identities (field objects, with looks
-- that keep the flip of what wears them).
local BOMB = countdown.variant {
    appear_anim = 0,
    idle_anim = 1,
    hp = 50,
    identity = define.identity {
        id = "timebomb",
        class = "field_object",
        version = 5,
        ai_index = 1,
        look = { sprite = asset.sprite("countdown-bomb"), anim = 1, palette = 0, shadow = true, keeps_flip = true },
    },
    class = 1,
    collision = BODY,
}
local BIG_BOMB = countdown.variant {
    appear_anim = 2,
    idle_anim = 3,
    hp = 200,
    identity = define.identity {
        id = "timebomb-big",
        class = "field_object",
        version = 0,
        ai_index = 1,
        look = { sprite = asset.sprite("countdown-bomb"), anim = 3, palette = 0, shadow = true, keeps_flip = true },
    },
    class = 1,
    collision = BODY,
    strong_shake = true,
}

local timebom1 = define.chip {
    id = "timebom1",
    codes = { "F", "G", "H" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 150,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 145, index = 145, sort = 434 },
    slot_in_limit = 3,
    dimming = controller.hook(BOMB),
}

local timebom2 = define.chip {
    id = "timebom2",
    codes = { "C", "D", "E" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 190,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 146, index = 146, sort = 434 },
    slot_in_limit = 3,
    dimming = controller.hook(BOMB),
}

local timebom3 = define.chip {
    id = "timebom3",
    codes = { "L", "M", "N" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 3,
    mb = 37,
    damage = 230,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    library = { number = 147, index = 147, sort = 435 },
    slot_in_limit = 3,
    dimming = controller.hook(BOMB),
}

local timebom = define.chip {
    id = "timebom",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 700,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    library = { number = 335, index = 16, sort = 0 },
    slot_in_limit = 1,
    program_advances = {
        { order = 28, code_run = { chip = timebom1, count = 3 } },
        { order = 29, code_run = { chip = timebom2, count = 3 } },
        { order = 30, code_run = { chip = timebom3, count = 3 } },
    },
    dimming = controller.hook(BIG_BOMB),
}

return { timebom1 = timebom1, timebom2 = timebom2, timebom3 = timebom3, timebom = timebom }
