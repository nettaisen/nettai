--!strict
-- BN5's Mine chip's dimming controller (effect object #0x29) and mine (attack object #0x4C): BN6's code
-- (@common/mine), BN5's kinds; the mine's identity, look and collision types BN6's.

local bn6_collision = require("@bn6/rules/collision")
local common = require("@common/mine/controller")
local land_mine = require("@common/mine/land_mine")

local mine = land_mine.make_kind {
    id = "bn5:mine/land-mine",
    identity = define.identity {
        id = "bn5:mine",
        class = "field_object",
        version = 1,
        ai_index = 1,
        absorbable = false,
        scrap = false,
        look = { anim = 0, palette = 0, shadow = false },
    },
    sprite = asset.sprite("bn6:land-mine"),
    hop_sound = asset.sound("bn6:hop"),
    blast = define.effect { sprite = asset.sprite("bn6:blast"), anim = 0, palette = 0 },
    blast_sound = asset.sound("bn6:hit-bomb-1"),
    self_type = bn6_collision.mine,
    target = bn6_collision.hits_both_sides,
}

return common.make { id = "bn5:mine/controller", mine = mine }
