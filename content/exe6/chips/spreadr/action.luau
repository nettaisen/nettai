--!strict
-- The Spreaders' action (the original's action 0x25, `sub_80ECBB0`;
-- docs/engine/shot-chips.md §5): the shared action
-- (@exelib/spreadr/action) with EXE6's sound, attachment and bullet.
-- SpoutCross's charged shot fires its own bullet from its water cannon the
-- same way.

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local common = require("@exelib/spreadr/action")
local navi_arms = require("../../lib/navi_arms")

local spreader = {}

-- One state for every spreader.
spreader.STATE = common.STATE

spreader.guns = {
    spreader = attachment.look { sprite = asset.sprite("spreader"), palette = 0, attach_point = 11 },
    water_cannon = attachment.look { sprite = asset.sprite("water-cannon"), palette = 0, attach_point = 25 },
}

local LOOK: SpreaderLook = {
    sound = asset.sound("h-burst"),
    attachment = attachment.kind,
    bullet_kind = bullet.kind,
    gun_anims = navi_arms.spreader,
}

-- A chip's spreader.
function spreader.action(spec: SpreaderSpec): Action
    return common.action(spec, LOOK)
end

return spreader
