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
    -- In the overlay, its animation 0.
    hold = function(me: Object, look: AttachmentLook, which: Slot)
        attachment.attach(me, look, which)
    end,
    slot = "overlay",
    sound = asset.sound("h-burst"),
    burst = burst.kind,
    -- The navi aims in its animation 0xB.
    anim = 0x0B,
}

-- An H-Burst whose shot bursts `spec.bursts` times.
function action.action(spec: HyperBurstSpec): Action
    return common.action(spec, LOOK)
end

return action
