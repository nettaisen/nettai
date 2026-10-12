--!strict
-- ShotStar (EXE4 chip 0x0D2): a dimming chip that drops thirty meteors on the enemy's area, one every 10 ticks, going
-- through its panels from the far column: the dimming chips' action 0x0C, its variant 0x19 (0x080220E0's). Its
-- controller (effect object #0x34, 0x080E5C90; its spawner 0x080E5D78 the usual one) is the shower EXE6's and EXE5's
-- Meteors chips kept (@exelib/meteors/controller), finding each meteor's panel as the last one falls (0x080E5D9A:
-- `scanned`) where theirs list the panels once. Its meteors are EXE4's (chips/meteors' kind, attack object #0x56) of
-- row 1 (0x080D8586: thrown at navis, hit modifier 3, the camera shaking, no panel change), running while dimmed
-- (0x080D879A). Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local controller = require("@exelib/meteors/controller")
local falling_meteor = require("@exelib/meteors/falling_meteor")
local meteors = require("./meteors/meteors")

-- Row 1 of 0x080D8580 (hit modifier 3: 0x080D863E).
local ROW = falling_meteor.row { is = collision.thrown, hits = collision.hits_navis, hit_mod = 3, shake = 1, panel = "none" }

local DIMMING = controller.make {
    id = "shotstar/controller",
    meteor = meteors.meteor,
    row = ROW,
    targets = controller.scanned,
}

local chip: Chip = {
    codes = { "R" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 4,
    mb = 73,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = DIMMING,
}

return { shotstar = chip }
