--!strict
-- TimeBom+ (EXE4 chip 0x15A): the Program Advance of three TimeBombs of consecutive codes (the recipe table, the
-- pointer list at 0x080212B0). TimeBomb's controller (./timebomb/bomb, the dimming chips' variant 0x11) with the
-- bomb of its first parameter, 1: animations 2 and 3, 300 HP, its blast shaking the camera a second time, harder
-- (0x080D6E42).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bomb = require("./timebomb/bomb")
local countdown = require("@exelib/timebom/countdown")
local timebombs = require("./timebomb")

local BIG_BOMB = countdown.variant {
    appear_anim = 2,
    idle_anim = 3,
    hp = 300,
    class = 1,
    collision = bomb.body,
    strong_shake = true,
}

local timebom: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 600,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    program_advances = { { order = 35, code_run = { chip = timebombs.timebomb, count = 3 } } },
    dimming = bomb.controller.hook(BIG_BOMB),
}

return { timebom = timebom }
