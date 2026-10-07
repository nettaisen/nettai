--!strict
-- The Spreaders' action (the original's action 0x25, `sub_80ECBB0`;
-- docs/engine/shot-chips.md §5): the shared action
-- (@exelib/spreadr/action) with EXE6's sound, attachment and bullet.
-- Each user states its gun and bullet (`spreader.action { gun = ..., bullet
-- = ... }`): the Spreaders' (./init); SpoutCross's charged shot and the
-- patch card puffy's fire theirs from the water cannon the same way.

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local common = require("@exelib/spreadr/action")
local navi_arms = require("../../lib/navi_arms")

local spreader = {}

-- One state for every spreader.
spreader.STATE = common.STATE

local LOOK: SpreaderLook = {
    sound = asset.sound("h-burst"),
    -- Held in the overlay, its animation by the holder's AI index (the arms' table).
    hold = attachment.holder(navi_arms.spreader),
    slot = "overlay",
    bullet_kind = bullet.kind,
    -- The navi fires in its animation 0xB.
    anim = 0xB,
}

-- A chip's spreader.
function spreader.action(spec: SpreaderSpec): Action
    return common.action(spec, LOOK)
end

return spreader
