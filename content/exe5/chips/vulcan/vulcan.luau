--!strict
-- EXE5's Vulcans (action 0x19, 0x080EC5D0): the shared vulcan (@exelib/vulcan/action, EXE6's code) holding EXE5's gun (the
-- attachment table's row 0xB, 0c-1d: EXE6's row 0xD) and firing EXE5's bullets (objects/bullet). The bullets by the
-- chip's subtype (0x080EC6B4: 3, 5 and 7 for Vulcan1 to Vulcan3, 12 for SuprVulc, 20, 24 and 28 for InfVulc1 to
-- InfVulc3), the variant its record's parameter (row 0xC; SuprVulc's 0x10, the InfVulcs' 0xE). From subtype 4 on
-- (the InfVulcs) the gun is the attachment table's row 0x1F (0x080EC608: the same sheet in palette 3).

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

-- The InfVulcs' gun (row 0x1F).
local INFINITE_GUN = attachment.look { sprite = asset.sprite("vulcan"), palette = 3, attach_point = 14 }

-- The vulcan of a chip that fires `shots` bullets (0x080EC6B4 by subtype: SuprVulc's 12), of `variant` (the
-- Vulcans' when none; SuprVulc's row 0x10).
function vulcan.action(shots: number, variant: BulletVariant?): Action
    return vulcan_action.action({ shots = shots, bullet = variant or bullet.variants.vulcan }, LOOK)
end

-- An InfVulc's vulcan (subtypes 4 to 6), firing `shots` bullets.
function vulcan.infinite(shots: number): Action
    return vulcan_action.action({ shots = shots, bullet = bullet.variants.infinite, arm = INFINITE_GUN }, LOOK)
end

return vulcan
