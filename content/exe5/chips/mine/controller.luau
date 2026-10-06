--!strict
-- EXE5's Mine chip's dimming controller (effect object #0x29) and mine (attack object #0x4C): EXE6's code
-- (@exelib/mine), EXE5's kinds; the mine's identity and look EXE6's, its collision types EXE5's (EXE6's values).

local collision = require("../../rules/collision")
local common = require("@exelib/mine/controller")
local land_mine = require("@exelib/mine/land_mine")

local mine = land_mine.make_kind {
    id = "mine/land-mine",
    identity = new.identity {
        class = "field_object",
        version = 1,
        ai_index = 1,
        absorbable = false,
        scrap = false,
        throwable = false,
        look = { anim = 0, palette = 0, shadow = false },
    },
    sprite = asset.sprite("land-mine"),
    hop_sound = asset.sound("hop"),
    blast = new.effect { sprite = asset.sprite("blast"), anim = 0, palette = 0 },
    blast_sound = asset.sound("hit-bomb-1"),
    self_type = collision.mine,
    target = collision.hits_both_sides,
}

return common.make { id = "mine/controller", mine = mine }
