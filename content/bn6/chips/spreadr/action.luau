--!strict
-- The Spreaders' action (the original's action 0x25, `sub_80ECBB0`;
-- docs/engine/shot-chips.md §5): the shared action
-- (@exelib/spreadr/action) with BN6's sound, attachment and bullet.
-- SpoutCross's charged shot fires its own bullet from its water cannon the
-- same way.

local attachment = require("../../objects/attachment/attachment")
local bullet = require("../../objects/bullet/bullet")
local common = require("@exelib/spreadr/action")

local spreader = {}

-- One state for every spreader.
spreader.STATE = common.STATE

spreader.guns = {
    spreader = attachment.look { sprite = asset.sprite("bn6:spreader"), palette = 0, attach_point = 11 },
    water_cannon = attachment.look { sprite = asset.sprite("bn6:water-cannon"), palette = 0, attach_point = 25 },
}

local LOOK: SpreaderLook = { sound = asset.sound("bn6:h-burst"), attachment = attachment.kind, bullet_kind = bullet.kind }

-- A chip's spreader.
function spreader.action(spec: SpreaderSpec): Action
    return common.action(spec, LOOK)
end

return spreader
