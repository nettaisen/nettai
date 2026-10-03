--!strict
-- AirSpin's whirlwind top (BN5's attack object #0x9B, 0x080D9EE4): BN6's
-- (@common/airspin/top) in BN5's look. Its side's own body of 50 HP that
-- the other side hits (BN6's is a neutral one of 400), set down wherever it
-- is sent; a wind hit doesn't spin it more and no chip removes it; its gust
-- (effect 0x4F, not turned to its side) hits without pushing. As it stands,
-- and after each spin, it draws RNG2 (bit 2) for the style, 9 or 11, of a
-- marker over its panel (lib/navi-chips/marker: battle flag 0x40's), for 20
-- ticks, then its interval and 3; its spins count down on Timer 2.
--
-- Not reached: 0x080DA194, after the gust's routine, which would turn its
-- panel to type 2, double its spins and hit every 5 ticks: nothing calls it.

local collision = require("../../rules/collision")
local common = require("@common/airspin/top")
local effects = require("@bn6/lib/effects")
local marker = require("../../lib/navi-chips/marker")

-- Its sprite: animation 0 standing, 1 spinning.
local SPRITE = asset.sprite("bn5:air-spin")

return common.make {
    id = "bn5:airspin/top",
    sprite = SPRITE,
    identity = define.identity {
        id = "bn5:airspin-top",
        class = "field_object",
        version = 0,
        ai_index = 1,
        look = { sprite = SPRITE, anim = 1, palette = 0, shadow = false },
    },
    hp = 0x32,
    body = collision.own_body,
    hit_by = collision.hit_by_other_side,
    -- The spin's gust (the effect table's row 0x4F), and its palette offset.
    gust = define.effect { sprite = asset.sprite("bn5:moon-blade"), anim = 0, palette = 0 },
    gust_palette = 2,
    gust_side = false,
    gust_self = collision.attack,
    gust_target = collision.hits_navis,
    puff = effects.puff,
    explosion = effects.explosion,
    sounds = {
        set_down = asset.sound("bn5:set-down"),
        spin = asset.sound("bn5:spin"),
        hit_bomb = asset.sound("bn5:hit-bomb-1"),
    },
    set_down_check = false,
    extra_spins = false,
    absorbed = nil,
    -- 0x080E9FD2: the marker turned to its facing too.
    marker = function(owner: Object, x: number, y: number, ticks: number, style: number)
        marker.spawn(owner, x, y, ticks, style, true)
    end,
    spins_on_timer2 = true,
}
