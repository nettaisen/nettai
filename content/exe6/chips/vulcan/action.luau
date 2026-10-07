--!strict
-- The Vulcans' action (the original's action 0x17, `sub_80EBF10`;
-- docs/engine/shot-chips.md §4): the shared action
-- (@exelib/vulcan/action) with EXE6's sound, vulcan, attachment and bullet.
-- Each chip states its own bullets (`vulcan.action { shots = ..., bullet =
-- ... }`).

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local common = require("@exelib/vulcan/action")

local vulcan = {}

-- One state for the four vulcans.
vulcan.STATE = common.STATE

local LOOK: VulcanLook = {
    sound = asset.sound("col-army-2"),
    -- The vulcan (the original's attachment row 0xD).
    arm = attachment.look { sprite = asset.sprite("vulcan"), palette = 0, attach_point = 14 },
    -- Held in the overlay (`sub_80EBF30`), its animation 0.
    hold = function(me: Object, look: AttachmentLook, which: Slot)
        attachment.attach(me, look, which)
    end,
    slot = "overlay",
    bullet_kind = bullet.kind,
    -- The navi holds it in its animation 0xA and fires in 0xD.
    hold_anim = 0xA,
    fire_anim = 0xD,
}

-- A chip's vulcan.
function vulcan.action(spec: VulcanSpec): Action
    return common.action(spec, LOOK)
end

return vulcan
