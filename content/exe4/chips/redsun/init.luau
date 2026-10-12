--!strict
-- RedSun (EXE4 chip 0x130, Red Sun's Giga chip): the dimming chips' variant 0x2D: its user warps out and a red sun
-- (./sun) rises over the user's panel, throws five meteors (./meteor) at the panel three columns ahead and dives onto
-- it, cracking the panels around it; its controller is effect object #0x5A (../lib/sun_and_moon). Its damage, the
-- dive's, is formula 47 (0x080195CE): 100 and 40 for each ten of the side's `rival_wins`, up to 100.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local damage = require("../rules/sun_and_moon").damage
local sun = require("@self/sun")
local sun_and_moon = require("../lib/sun_and_moon")

local CONTROLLER = sun_and_moon.make { id = "redsun/controller", name = "RedSun", attack = sun.spawn }

local chip: Chip = {
    codes = { "R" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 90,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    damage = damage(100),
    dimming = CONTROLLER.hook,
    auto_battle = { class = "into_reach", range = "three_ahead" },
}

return { redsun = chip }
