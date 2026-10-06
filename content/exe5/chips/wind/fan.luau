--!strict
-- EXE5's fan for Wind and Fan (attack object #0x48, 0x080CE734): the shared one (@exelib/wind/fan, EXE6's code) in
-- EXE5's look (sprite 04-0A), with EXE5's gust (navis/megaman/gust) and EXE5's differences: leaving (0x080CE90E), any
-- removal (EXE5's flag word's 0x10000) leaves a puff, with no blinking out and no absorbing; Fan's gust (0x080CE96C)
-- starts on the far column alone, none when another obstacle stands there. It takes hits as EXE6's (0x08017984: a
-- push keeps the damage, after EXE5's lava burn). Its actions are EXE6's numbers (EXE5's table, 0x080CE810, has six
-- shared entries, its rising and blowing EXE5's 6 and 7 at EXE6's 8 and 9, the same actions).

local collision = require("../../rules/collision")
local common = require("@exelib/wind/fan")
local gust = require("../../navis/megaman/gust")

local fan = {}

fan.SPRITE = asset.sprite("fan")

fan.variant = common.variant

-- The effects (EXE5's table, 0x080DFC94): row 0x14 (14-0D, animation 2) when removed, row 0 (14-00) when broken.
local made = common.make {
    id = "wind/fan",
    sprite = fan.SPRITE,
    appear_sound = asset.sound("appear"),
    blow_sound = asset.sound("dustbrk"),
    break_sound = asset.sound("hit-bomb-1"),
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    is = collision.own_body,
    hits = collision.hit_by_other_side,
    spawn_gust = gust.spawn,
    absorbs = false,
    far_column_only = true,
}

fan.kind = made.kind
fan.spawn = made.spawn

return fan
