--!strict
-- PitHoky (BN5 chip 0x155): the Program Advance of three AirHocs with consecutive codes (or two CannBalls and an
-- AirHoc): AirHoc's flick (action 0x21) of BN5's puck in row 3 (chips/airhoc/puck: 18 steps, a trail), down the field (the
-- record's Param1 3 and Param2 1).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local flick = require("@common/airhocky/flick")
local puck = require("../airhoc/puck")
local recipe_airhoc = require("../airhoc/chip")
local recipe_cannball = require("../cannball/chip")

return define.chip {
    id = "bn5:pithoky",
    codes = {},
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 5,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 19, sequence = { recipe_cannball, recipe_cannball, recipe_airhoc }, per_player_gauges_only = true },
        { order = 55, code_run = { chip = recipe_airhoc, count = 3 } },
    },
    action = flick.action {
        id = "bn5:pithoky/action",
        kind = puck.kind,
        puck = puck.variants.pithoky,
        down = true,
    },
}
