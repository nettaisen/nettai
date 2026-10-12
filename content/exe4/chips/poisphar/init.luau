--!strict
-- PoisPhar (EXE4 chip 0x15B): the Program Advance of Hole, DrkLine and Anubis in that order (the recipe table, the
-- pointer list at 0x080212B0). Anubis's controller (./anubis/anubis, the dimming chips' variant 0x18) with the statue
-- of its first parameter, 1 (0x080D82E8's row 1): animation 1, a drain every tick, 300 HP.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local anubis = require("./anubis/anubis")
local recipe_anubis = require("./anubis").anubis
local recipe_drkline = require("./drkline").drkline
local recipe_hole = require("./hole").hole

local STATUE = anubis.variant { anim = 1, period = 1, hp = 300, lifetime = 3000 }

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 0,
    hit_param = 129,
    lockout = 0,
    flags = { "dimming", "library" },
    program_advances = { { order = 6, sequence = { recipe_hole, recipe_drkline, recipe_anubis } } },
    dimming = anubis.controller.hook(STATUE),
}

return { poisphar = chip }
