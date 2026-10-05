--!strict
-- EXE5's ColorPt and DblPoint controller and points (the dimming handler's subtype 30: effect objects #0x50,
-- 0x080E650C, and #0x51, 0x080E65E8): EXE6's (@exelib/colorpt) but for where a point's bonus goes when the side has
-- a special chip's (0x080E6746): with each side keeping its own gauge (the operation battle, never in a netbattle: then
-- the side's special bonus), where EXE6's tests its player's AI+0xBB; and EXE5's look.

local controller = require("@exelib/colorpt/controller")
local point = require("@exelib/colorpt/point")
local trajectory = require("../../lib/trajectory")
local operation_battle = require("../../lib/operation_battle")

local points = point.make {
    id = "colorpt/point",
    sprite = asset.sprite("grab-shot"),
    sounds = {
        appear = asset.sound("point-appear"),
        rise = asset.sound("point-rise"),
        arrive = asset.sound("cross-merge"),
    },
    special = function(_side: number, _navi: Object): boolean
        return operation_battle()
    end,
    trajectory = trajectory,
}

return controller.make { id = "colorpt/controller", point = points.spawn }
