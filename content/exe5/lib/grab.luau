--!strict
-- EXE5's AreaGrab and PanelGrab: their grab shot (attack object #0xF, EXE6's code: @exelib/grab/shot), EXE6's look (EXE6's
-- sheet and sound numbers) and EXE5's collision types, and their dimming controller (effect object #3:
-- @exelib/grab/controller), EXE5's kinds. Each chip's `dimming` hook is `grab.area` or `grab.panel`.

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
    spares = "last_full_column",
}

return controller.make {
    id = "grab/controller",
    shot = shot_kind,
    column = controller.column_to_grab,
    panel = controller.edge_panel,
}
