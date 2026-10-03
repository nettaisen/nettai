--!strict
-- BN5's thunder ball (attack object 0x2A, BN6's number; 0x080C9FA0, BN6's `sub_80C940C`): BN6's ball
-- (@bn6/chips/thunder/ball) with BN5's look: BN5's ball sprite (10-11, a fast or 12-panel ball's 0c-53), animation
-- 0, collision type row 0x0A (a 12-panel ball's, DarkThnd's, row 0x2B) with hit modifier 0 (BN6's an attack with
-- 1), and its sound for every ball. Thunder's and DarkThnd's.

local ball = require("@bn6/chips/thunder/ball")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local thunder = {}

thunder.kind = ball.make_kind("bn5:thunder/ball", {
    sprite = asset.sprite("bn5:thunder-ball"),
    small_anim = 0,
    big_sprite = asset.sprite("bn5:big-thunder-ball"),
    big_anim = 0,
    is = collision.thrown,
    dark_is = collision.collision_2b,
    hit_mod = 0,
    spark = sparks.elec,
    sound = asset.sound("bn5:erase-man-2"),
})

return thunder
