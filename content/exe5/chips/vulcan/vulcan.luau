--!strict
-- EXE5's Vulcans (action 0x19, 0x080EC5D0): the shared vulcan (@exelib/vulcan/action, EXE6's code) holding EXE5's gun (the
-- attachment table's row 0xB, 0c-1d: EXE6's row 0xD) and firing EXE5's bullets (objects/bullet). Each chip states its
-- own (`vulcan.action { shots = ..., bullet = ..., arm = ... }`): its bullets by its subtype (0x080EC6B4: 3, 5 and 7
-- for Vulcan1 to Vulcan3, 12 for SuprVulc, 20, 24 and 28 for InfVulc1 to InfVulc3), its bullet's variant the row its
-- record's parameter names (the Vulcans' 0xC, SuprVulc's 0x10, the InfVulcs' 0xE), and from subtype 4 on (the InfVulcs)
-- its gun (0x080EC608: the attachment table's row 0x1F).

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local vulcan_action = require("@exelib/vulcan/action")

local vulcan = {}

-- EXE5's gun, attachment and bullet; the shot's sound EXE6's number.
local LOOK: VulcanLook = {
    sound = asset.sound("col-army-2"),
    arm = attachment.look { sprite = asset.sprite("vulcan"), palette = 0, attach_point = 14 },
    -- Held in the overlay, its animation 0, as EXE6's.
    hold = function(me: Object, look: AttachmentLook, which: Slot)
        attachment.attach(me, look, which)
    end,
    slot = "overlay",
    bullet_kind = bullet.kind,
    -- The navi holds it in its animation 0xA and fires in 0xD, as EXE6's.
    hold_anim = 0xA,
    fire_anim = 0xD,
}

-- A chip's vulcan.
function vulcan.action(spec: VulcanSpec): Action
    return vulcan_action.action(spec, LOOK)
end

return vulcan
