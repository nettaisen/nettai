--!strict
-- EXE4's hockey puck (attack object #0x2E, 0x080D2788): the shared puck (@exelib/airhocky/puck, EXE5's code) with
-- EXE4's look: EXE5's sprite (10-12), collision types and spark, EXE4's sounds (0xC0 flicked, 0xCC bouncing), its burst
-- a puff (effect row 0x15). Its rows (0x080D2768: the steps it lasts, how it bounces, its ticks per step) are its
-- chips': AirHoc1 to 3 state theirs (rows 6 to 8). (Row 9 would leave a trail, 0x080D2ACC: no chip's is.)

local collision = require("../../rules/collision")
local common = require("@exelib/airhocky/puck")
local sparks = require("../../lib/sparks")

local puck = {}

puck.kind = common.make_kind("airhoc/puck", {
    sprite = asset.sprite("hockey-puck"),
    sound = asset.sound("hockey-puck"),
    bounce = asset.sound("bounce"),
    is = collision.breaking,
    hits = collision.hits_navis,
    spark = sparks.breaking,
    burst = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
})

-- A puck's row.
puck.variant = common.variant

return puck
