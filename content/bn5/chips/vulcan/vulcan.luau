--!strict
-- BN5's Vulcans (action 0x19, 0x080EC5D0): the shared vulcan (@exelib/vulcan/action, BN6's code) holding BN5's gun (the
-- attachment table's row 0xB, 0c-1d: BN6's row 0xD) and firing BN5's bullets (objects/bullet). The bullets by the
-- chip's subtype (0x080EC6B4: 3, 5 and 7 for Vulcan1 to Vulcan3, 12 for SuprVulc, 20, 24 and 28 for InfVulc1 to
-- InfVulc3), the variant its record's parameter (row 0xC; SuprVulc's 0x10, the InfVulcs' 0xE). From subtype 4 on
-- (the InfVulcs) the gun is the attachment table's row 0x1F (0x080EC608: the same sheet in palette 3).

local attachment = require("../../objects/attachment/attachment")
local bullet = require("../../objects/bullet/bullet")
local vulcan_action = require("@exelib/vulcan/action")

local vulcan = {}

-- BN5's gun, attachment and bullet; the shot's sound BN6's number.
local LOOK: VulcanLook = {
    sound = asset.sound("bn5:col-army-2"),
    arm = attachment.look { sprite = asset.sprite("bn5:vulcan"), palette = 0, attach_point = 14 },
    attachment = attachment.kind,
    bullet_kind = bullet.kind,
}

-- The InfVulcs' gun (row 0x1F).
local INFINITE_GUN = attachment.look { sprite = asset.sprite("bn5:vulcan"), palette = 3, attach_point = 14 }

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
