--!strict
-- TimeBom1, TimeBom2, TimeBom3 and TimeBom+: the timebom series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local countdown = require("@exelib/timebom/countdown")
local timebom = require("@self/timebom")

-- What the bombs are and what they react to: a neutral body, hit by everything (rows 0xE and 0xF).
local BODY = { is = collision.neutral_body, hits = collision.everything }

-- TimeBom1-3's bomb (0x080CEEE4's row 0).
local BOMB = countdown.variant {
    appear_anim = 0,
    idle_anim = 1,
    hp = 50,
    identity = new.identity {
        class = "field_object",
        version = 5,
        ai_index = 0x21,
        look = { sprite = asset.sprite("countdown-bomb"), anim = 1, palette = 0, shadow = true, keeps_flip = true },
    },
    class = 1,
    collision = BODY,
}

-- TimeBom+'s (row 1).
local BIG_BOMB = countdown.variant {
    appear_anim = 2,
    idle_anim = 3,
    hp = 300,
    identity = new.identity {
        class = "field_object",
        version = 0,
        ai_index = 0x21,
        look = { sprite = asset.sprite("countdown-bomb"), anim = 3, palette = 0, shadow = true, keeps_flip = true },
    },
    class = 1,
    collision = BODY,
    strong_shake = true,
}

-- TimeBom1 (EXE5 chip 0x04b): a bomb on the enemy's frontmost panel of a random row that counts 3, 2, 1 and blows
-- up the whole enemy area unless it is broken first (chips/timebom/timebom: EXE6's TimeBom with EXE5's placement).
local timebom1: Chip = {
    codes = { "H", "N", "T" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 0,
    mb = 32,
    damage = 100,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = timebom.controller.hook(BOMB),
    auto_battle = { class = "at_once" },
}

-- TimeBom2 (EXE5 chip 0x04c): a bomb on the enemy's frontmost panel of a random row that counts 3, 2, 1 and blows
-- up the whole enemy area unless it is broken first (chips/timebom/timebom: EXE6's TimeBom with EXE5's placement).
local timebom2: Chip = {
    codes = { "D", "H", "Z" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 54,
    damage = 140,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = timebom.controller.hook(BOMB),
    auto_battle = { class = "at_once" },
}

-- TimeBom3 (EXE5 chip 0x04d): a bomb on the enemy's frontmost panel of a random row that counts 3, 2, 1 and blows
-- up the whole enemy area unless it is broken first (chips/timebom/timebom: EXE6's TimeBom with EXE5's placement).
local timebom3: Chip = {
    codes = { "F", "H", "S" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 76,
    damage = 200,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = timebom.controller.hook(BOMB),
    auto_battle = { class = "at_once" },
}

-- TimeBom+ (EXE5 chip 0x15b): the Program Advance of TimeBom1, TimeBom2 and TimeBom3 in that order: TimeBom1-3's bomb
-- with 300 HP, its own look and a harder shake (chips/timebom/timebom: EXE6's TimeBom with EXE5's placement).

local recipe_timebom1 = timebom1
local recipe_timebom2 = timebom2
local recipe_timebom3 = timebom3

local timebom_chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 600,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 23, sequence = { recipe_timebom1, recipe_timebom2, recipe_timebom3 } },
    },
    dimming = timebom.controller.hook(BIG_BOMB),
    auto_battle = { class = "at_once" },
}

return { timebom1 = timebom1, timebom2 = timebom2, timebom3 = timebom3, timebom = timebom_chip }
