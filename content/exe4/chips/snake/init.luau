--!strict
-- Snake (EXE4 chip 0x060): the dimming chips' variant 0x17, snakes leaping out of the holes on the user's side at the
-- nearest enemy, three at a time: EXE5's Snake (@exelib/snake/batch: its controller, effect object #0x32, 0x080E5B50;
-- its nest, attack object #0x53, 0x080D7DB8) with EXE4's snakes (attack object #0x54, 0x080D80E4: @exelib/snake):
-- sheet 0c-2c, rising with 0xA0 and leaping with 0x91, waiting 48 ticks, striking null with spark 4 and hit modifier 1
-- (0x080D8224), their targets picked with EXE4's slip (0x080D7F70, `best_row_slip`).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local batch = require("@exelib/snake/batch")
local collision = require("../rules/collision")
local common = require("@exelib/snake")
local sparks = require("../lib/sparks")
local trajectory = require("../lib/trajectory")

local SNAKE = common.make_kind("snake/snake", {
    trajectory = trajectory,
    sprite = asset.sprite("snake"),
    rise_sound = asset.sound("tops"),
    leap_sound = asset.sound("zap-ring"),
    wait = 0x30,
    strike_element = 0,
    strike_spark = sparks.wood,
    strike_hit_mod = 1,
    is = collision.attack,
    hits = collision.hits_navis,
    joined = function(_me: Object) end,
    -- 0x080D8268: its flag cleared.
    left = batch.left,
})

local MADE = batch.make { nest_id = "snake/nest", controller_id = "snake/controller", snake = SNAKE, best_row_slip = true }

local chip: Chip = {
    codes = { "D", "M", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 60,
    damage = 20,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = MADE.hook,
}

return { snake = chip }
