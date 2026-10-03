--!strict
-- PoisPhar (BN5 chip 0x156): the Program Advance of BugBomb, Geddon3 and Anubis in that order: a statue in front of
-- the user that drains every enemy navi 1 HP each tick for 3000 ticks, with bubbles over the enemy's area: Anubis's
-- statue (objects/anubis, the dimming handler's subtype 17) in its second variant (the chip's first parameter, 1),
-- with 300 HP.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local anubis = require("../../objects/anubis/anubis")
local recipe_anubis = require("../anubis/chip")
local recipe_bugbomb = require("../bugbomb/chip")
local recipe_geddon3 = require("../geddon3/chip")

return define.chip {
    id = "bn5:poisphar",
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
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 24, sequence = { recipe_bugbomb, recipe_geddon3, recipe_anubis } },
    },
    dimming = anubis.controller.hook(anubis.variants.poisphar),
}
