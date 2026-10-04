--!strict
-- BN5's Mine chip's dimming controller (effect object #0x29) and mine (attack object #0x4C): BN6's code
-- (@exelib/mine), BN5's kinds; the mine's identity and look BN6's, its collision types BN5's (BN6's values).

local collision = require("../../rules/collision")
local common = require("@exelib/mine/controller")
local land_mine = require("@exelib/mine/land_mine")

local mine = land_mine.make_kind {
    id = "bn5:mine/land-mine",
    identity = define.identity {
        id = "bn5:mine",
        class = "field_object",
        version = 1,
        ai_index = 1,
        absorbable = false,
        scrap = false,
        throwable = false,
        look = { anim = 0, palette = 0, shadow = false },
    },
    sprite = asset.sprite("bn5:land-mine"),
    hop_sound = asset.sound("bn5:hop"),
    blast = define.effect { sprite = asset.sprite("bn5:blast"), anim = 0, palette = 0 },
    blast_sound = asset.sound("bn5:hit-bomb-1"),
    self_type = collision.mine,
    target = collision.hits_both_sides,
}

return common.make { id = "bn5:mine/controller", mine = mine }
