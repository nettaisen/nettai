--!strict
-- The fan Wind and Fan place (the original's attack object #0x48, `sub_80CCFDC`; docs/engine/dimming-chip-effects.md
-- §1): the shared one (@exelib/wind/fan) with EXE6's look and gust (objects/gust).

local absorbed_obstacle = require("../../objects/absorbed-obstacle")
local collision = require("../../rules/collision")
local common = require("@exelib/wind/fan")
local effects = require("../../lib/effects")
local gust = require("../../objects/gust")

local fan = {}

local SPRITE = asset.sprite("fan")
-- How a fan looks absorbed (both variants' obstacle kind 0: its own sprite).
fan.absorbed = absorbed_obstacle.look { sprite = SPRITE }

fan.variant = common.variant

local made = common.make {
    id = "wind/fan",
    sprite = SPRITE,
    appear_sound = asset.sound("appear"),
    blow_sound = asset.sound("dustbrk"),
    break_sound = asset.sound("hit-bomb-1"),
    puff = effects.puff,
    explosion = effects.explosion,
    is = collision.own_body,
    hits = collision.hit_by_other_side,
    spawn_gust = gust.spawn,
    absorbs = true,
}

fan.kind = made.kind
fan.spawn = made.spawn

return fan
