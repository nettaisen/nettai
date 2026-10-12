--!strict
-- EXE4's ColorPt and DblPoint controller and points (the dimming chips' variant 0x27: effect objects #0x50, 0x080E7BF0,
-- and #0x51, 0x080E7CE0): EXE6's and EXE5's (@exelib/colorpt) in EXE4's look, but for three things of EXE4's own:
--
-- - The controller's spawn (0x080E7C58) takes the user's side alone.
-- - A row's point leaves from the front of the user's area found from the far edge (0x080E7C7A: in row y,
--   `object_getClosestPanelMatchingRowFiltered` from the far edge of the user's side, for the first panel of the user's,
--   0x080E7CCC's filter: owner bit 0x20 forbidden for side 0, required for side 1), unless that is the user's back
--   column (column 1 for side 0, 6 for side 1), where EXE6's and EXE5's ask whether the user may lose the panel.
-- - A point's spawn (0x080E7E96) leaves 0xD9 in its element, the low byte of the routine address its controller's
--   panel test (0x08009EAC, through 0x08009ABC's `bx r2`) leaves in r2.
--
-- Its points add their bonus to the chip at the hand's cursor unless event flag 0x1187 is on (0x080E7E38: 0x080406A4),
-- which no netbattle sets: the side's special bonus is never theirs. Their sounds: 0xCE as one appears, 0xFA as it
-- rises, 0x10E as it reaches the navi.

local controller = require("@exelib/colorpt/controller")
local panels = require("@exelib/panels")
local point = require("@exelib/colorpt/point")
local trajectory = require("../../lib/trajectory")

-- The owner bit side 1's panels have (0x080E7CCC's filter: side 1's require it, side 0's forbid it).
local OWNER_1 = 0x20
-- 0x080E7EAA: the element the point's spawn leaves.
local ELEMENT = 0xD9

-- 0x080E7C7A: the front of the user's area in row `y`, not its back column; 0 for none.
local function front(me: Object, y: number): number
    local side = me.alliance
    local require, forbid = if side == 1 then OWNER_1 else 0, if side == 1 then 0 else OWNER_1
    local x = panels.closest_in_row(side, y, require, forbid)
    if x == side * 5 + 1 then
        return 0
    end
    return x
end

local points = point.make {
    id = "colorpt/point",
    sprite = asset.sprite("grab-shot"),
    sounds = {
        appear = asset.sound("point-appear"),
        rise = asset.sound("point-rise"),
        arrive = asset.sound("point-arrive"),
    },
    special = function(_side: number, _navi: Object): boolean
        return false
    end,
    trajectory = trajectory,
    element = ELEMENT,
}

return controller.make { id = "colorpt/controller", point = points.spawn, front = front, side_only = true }
