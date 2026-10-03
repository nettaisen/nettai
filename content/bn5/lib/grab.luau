--!strict
-- BN5's AreaGrab and PanelGrab: their grab shot (attack object #0xF, BN6's code: @common/grab/shot), BN6's look and
-- collision types, and their dimming controller (effect object #3: @common/grab/controller), BN5's kinds. Each
-- chip's `dimming` hook is `grab.area` or `grab.panel`.

local bn6_collision = require("@bn6/rules/collision")
local controller = require("@common/grab/controller")
local shot = require("@common/grab/shot")

local shot_kind = shot.make_kind {
    id = "bn5:grab/shot",
    sprite = asset.sprite("bn6:grab-shot"),
    fall_sound = asset.sound("bn6:grab-shot-2"),
    land_sound = asset.sound("bn6:grab-shot"),
    target = bn6_collision.hits_navis,
    self_type = bn6_collision.thrown,
}

return controller.make { id = "bn5:grab/controller", shot = shot_kind }
