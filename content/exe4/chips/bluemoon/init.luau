--!strict
-- BlueMoon (EXE4 chip 0x135, Blue Moon's Giga chip): the dimming chips' variant 0x2E: its user warps out and a blue
-- moon (./moon) rises over the user's panel and shines its ray (./ray) on the panel three columns ahead, hitting it
-- and stripping the navi there of its powers; its controller is effect object #0x5B (../lib/sun_and_moon). Its
-- damage, the ray's hit's, is formula 48 (0x08019606): 200 and 40 for each ten of the side's `rival_wins`, up to 100.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local damage = require("../rules/sun_and_moon").damage
local moon = require("@self/moon")
local sun_and_moon = require("../lib/sun_and_moon")

local CONTROLLER = sun_and_moon.make { id = "bluemoon/controller", name = "BlueMoon", attack = moon.spawn }

local chip: Chip = {
    codes = { "B" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 90,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    damage = damage(200),
    dimming = CONTROLLER.hook,
    auto_battle = { class = "into_reach", range = "three_ahead" },
}

return { bluemoon = chip }
