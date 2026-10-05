--!strict
-- EXE5's hockey puck (attack object #0x2E, 0x080CA3D4): the shared puck (@exelib/airhocky/puck, EXE6's code) with EXE5's
-- look: EXE6's collision types, spark and sounds, EXE5's sprite (10-12), its burst a puff (effect 0x14). AirHoc's and
-- PitHoky's. Its rows (0x080CA3A4) are EXE6's: the steps it lasts, how it bounces and its ticks per step; row 3
-- (PitHoky's) leaves a trail.

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
    burst = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    trail_tint = 0x8318,
})

-- The rows the chips name (0x080CA3A4): AirHoc's 2 (10 steps), PitHoky's 3 (18, with a trail).
puck.variants = {
    airhoc = common.variant { steps = 0x0A, own_side = 1, ticks = 6 },
    pithoky = common.variant { steps = 0x12, own_side = 1, ticks = 6, trail = true },
}

return puck
