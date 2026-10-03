--!strict
-- BN5's Vulcans (action 0x19, 0x080EC5D0): the shared vulcan (@common/vulcan/action, BN6's code) holding BN5's gun (the
-- attachment table's row 0xB, 0c-1d: BN6's row 0xD) and firing BN5's bullets (objects/bullet). The bullets by the
-- chip's subtype (0x080EC6B4: 3, 5 and 7 for Vulcan1 to Vulcan3), the variant its record's parameter (row 0xC).
-- (The InfVulcs' gun is row 0x1F, by subtype 4 and up.)

local attachment = require("../attachment/attachment")
local bullet = require("../bullet/bullet")
local vulcan_action = require("@common/vulcan/action")

local vulcan = {}

-- BN5's gun, attachment and bullet; the shot's sound BN6's.
local LOOK: VulcanLook = {
    sound = asset.sound("bn6:col-army-2"),
    arm = attachment.look { sprite = asset.sprite("bn5:vulcan"), palette = 0, attach_point = 14 },
    attachment = attachment.kind,
    bullet_kind = bullet.kind,
}

-- The vulcan of a chip that fires `shots` bullets (0x080EC6B4 by subtype: SuprVulc's 12), of `variant` (the
-- Vulcans' when none; SuprVulc's row 0x10).
function vulcan.action(shots: number, variant: BulletVariant?): Action
    return vulcan_action.action({ shots = shots, bullet = variant or bullet.variants.vulcan }, LOOK)
end

return vulcan
