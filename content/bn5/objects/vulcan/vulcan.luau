--!strict
-- BN5's Vulcans (action 0x19, 0x080EC5D0): BN6's vulcan (@bn6/chips/vulcan/action) holding BN5's gun (the
-- attachment table's row 0xB, 0c-1d: BN6's row 0xD) and firing BN5's bullets (objects/bullet). The bullets by the
-- chip's subtype (0x080EC6B4: 3, 5 and 7 for Vulcan1 to Vulcan3), the variant its record's parameter (row 0xC).
-- (The InfVulcs' gun is row 0x1F, by subtype 4 and up.)

local attachment = require("@bn6/objects/attachment/attachment")
local bullet = require("../bullet/bullet")
local vulcan_action = require("@bn6/chips/vulcan/action")

local vulcan = {}

local GUN = attachment.look { sprite = asset.sprite("bn5:vulcan"), palette = 0, attach_point = 14 }

-- The vulcan of a chip that fires `shots` bullets.
function vulcan.action(shots: number): Action
    return vulcan_action.action { shots = shots, bullet = bullet.variants.vulcan, arm = GUN, bullet_kind = bullet.kind }
end

return vulcan
