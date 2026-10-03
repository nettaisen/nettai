--!strict
-- BN5's ColorPt and DblPoint controller and points (the dimming handler's subtype 30: effect objects #0x50,
-- 0x080E650C, and #0x51, 0x080E65E8): BN6's (@common/colorpt) but for where a point's bonus goes when the side has
-- a special chip's (0x080E6746): with each side keeping its own gauge (battle flag 0x40, never in a netbattle: then
-- the side's special bonus), where BN6's tests its player's AI+0xBB; and BN5's look.

local controller = require("@common/colorpt/controller")
local point = require("@common/colorpt/point")
local trajectory = require("../../lib/trajectory")

local points = point.make {
    id = "bn5:colorpt/point",
    sprite = asset.sprite("bn5:grab-shot"),
    sounds = {
        appear = asset.sound("bn5:point-appear"),
        rise = asset.sound("bn5:point-rise"),
        arrive = asset.sound("bn5:cross-merge"),
    },
    special = function(_side: number, _navi: Object): boolean
        return battle.per_player_gauges()
    end,
    trajectory = trajectory,
}

return controller.make { id = "bn5:colorpt/controller", point = points.spawn }
