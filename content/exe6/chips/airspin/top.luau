--!strict
-- AirSpin's whirlwind top (attack object #0x9B, `sub_80D88E0`): the one EXE5
-- shares (@exelib/airspin/top) in EXE6's look. A field object (its side's
-- class-1 one) with 400 HP. Set down on a solid panel with no body on it,
-- it rolls forward at 4 pixels a tick toward the next panel while that one
-- is open (solid, nothing standing on it), at most 4 panels, then stands
-- 14 ticks and spins: it hits the ring around its panel (a one-tick hit
-- with the wind's gust) every 10 ticks, its spins' times, one more for
-- each wind hit it takes, and blows away. It breaks when its HP runs out
-- or a body, a neutral object or a type-2 attack touches it; a chip can
-- remove it (absorbed, or blinking out). It keeps running while dimmed,
-- but only acts outside dimming.

local absorbed_obstacle = require("../../objects/absorbed-obstacle")
local collision = require("../../rules/collision")
local common = require("@exelib/airspin/top")
local effects = require("../../lib/effects")

-- Its sprite: animation 0 standing, 1 spinning.
local SPRITE = asset.sprite("air-spin")

return common.make {
    id = "airspin/top",
    sprite = SPRITE,
    -- Its identity: a field object, with the look something that takes its
    -- place shows (DustMan's junk).
    identity = define.identity {
        id = "airspin-top",
        class = "field_object",
        version = 0,
        ai_index = 1,
        look = { sprite = asset.sprite("air-spin"), anim = 1, palette = 0, shadow = false },
    },
    hp = 0x190,
    body = collision.neutral_body,
    hit_by = collision.everything,
    -- The spin's gust (effect 0x43), and its palette offset.
    gust = define.effect { sprite = asset.sprite("moon-blade"), anim = 0, palette = 0 },
    gust_palette = 2,
    gust_side = true,
    gust_self = collision.pushing,
    gust_target = collision.hits_navis,
    puff = effects.puff,
    explosion = effects.explosion,
    sounds = {
        set_down = asset.sound("set-down"),
        spin = asset.sound("spin"),
        hit_bomb = asset.sound("hit-bomb-1"),
    },
    set_down_check = true,
    extra_spins = true,
    -- How it looks absorbed (the original's obstacle kind 0x0C: its own
    -- sprite).
    absorbed = absorbed_obstacle.look { sprite = SPRITE },
    marker = nil,
    spins_on_timer2 = false,
}
