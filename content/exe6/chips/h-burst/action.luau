--!strict
-- H-Burst (the original's action 0x34, `sub_80ED810`): the shared H-Burst
-- (@exelib/h_burst/action) with EXE6's spreader (attachment row 0x15), sound
-- and shot (./burst).

local attachment = require("../../objects/attachment")
local burst = require("./burst")
local common = require("@exelib/h_burst/action")

local action = {}

-- One state for every H-Burst action.
action.STATE = common.STATE

local LOOK: HyperBurstLook = {
    -- The spreader the navi holds (the original's attachment row 0x15).
    spreader = attachment.look { sprite = asset.sprite("spreader"), palette = 0, attach_point = 0x0B },
    attachment = attachment.kind,
    sound = asset.sound("h-burst"),
    burst = burst.kind,
}

-- An H-Burst whose shot bursts `spec.bursts` times.
function action.action(spec: HyperBurstSpec): Action
    return common.action(spec, LOOK)
end

return action
