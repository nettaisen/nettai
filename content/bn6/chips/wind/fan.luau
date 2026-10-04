--!strict
-- The fan Wind and Fan place (the original's attack object #0x48, `sub_80CCFDC`; docs/engine/dimming-chip-effects.md
-- §1): the shared one (@common/wind/fan) with BN6's look and gust (objects/gust).

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local common = require("@common/wind/fan")
local effects = require("../../lib/effects")
local gust = require("../../objects/gust/gust")

local fan = {}

local SPRITE = asset.sprite("bn6:fan")
-- How a fan looks absorbed (both variants' obstacle kind 0: its own sprite).
fan.absorbed = absorbed_obstacle.look { sprite = SPRITE }

fan.variant = common.variant

local made = common.make {
    id = "bn6:wind/fan",
    sprite = SPRITE,
    appear_sound = asset.sound("bn6:appear"),
    blow_sound = asset.sound("bn6:dustbrk"),
    break_sound = asset.sound("bn6:hit-bomb-1"),
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
