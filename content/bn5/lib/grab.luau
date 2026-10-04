--!strict
-- BN5's AreaGrab and PanelGrab: their grab shot (attack object #0xF, BN6's code: @exelib/grab/shot), BN6's look (BN6's
-- sheet and sound numbers) and BN5's collision types, and their dimming controller (effect object #3:
-- @exelib/grab/controller), BN5's kinds. Each chip's `dimming` hook is `grab.area` or `grab.panel`.

local collision = require("../rules/collision")
local controller = require("@exelib/grab/controller")
local shot = require("@exelib/grab/shot")

local shot_kind = shot.make_kind {
    id = "grab/shot",
    sprite = asset.sprite("grab-shot"),
    fall_sound = asset.sound("grab-shot-2"),
    land_sound = asset.sound("grab-shot"),
    target = collision.hits_navis,
    self_type = collision.thrown,
}

return controller.make { id = "grab/controller", shot = shot_kind }
