--!strict
-- PoisPhar (EXE5 chip 0x156): the Program Advance of BugBomb, Geddon3 and Anubis in that order: a statue in front of
-- the user that drains every enemy navi 1 HP each tick for 3000 ticks, with bubbles over the enemy's area: Anubis's
-- statue (chips/anubis/anubis, the dimming handler's subtype 17) in its second variant (the chip's first parameter, 1),
-- with 300 HP.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local anubis = require("./anubis/anubis")
local recipe_anubis = require("./anubis").anubis
local recipe_bugbomb = require("./bugbomb").bugbomb
local recipe_geddon3 = require("./geddon").geddon3

-- Its statue (row 1 of 0x080D0858): a drain every tick, 300 HP.
local STATUE = anubis.variant { anim = 1, period = 1, hp = 300, version = 3 }

local chip: Chip = {
    codes = {},
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 1,
    hit_param = 128,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 24, sequence = { recipe_bugbomb, recipe_geddon3, recipe_anubis } },
    },
    dimming = anubis.controller.hook(STATUE),
    auto_battle = { class = "landing_ahead" },
}

return { poisphar = chip }
