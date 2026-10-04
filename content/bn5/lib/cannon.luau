--!strict
-- BN5's cannons' action (the original's action 0x14, BN6's code: @exelib/cannon) with BN5's attachment and
-- projectile, its arms (the attachment table's rows 0 to 2: sprite 0c-01 in palettes 0 to 2) and its sound (0xAE).

local attachment = require("../objects/attachment/attachment")
local common = require("@exelib/cannon")
local projectile = require("./projectile")

local cannon = {}

local CANNON = asset.sprite("bn5:cannon")
cannon.arms = {
    cannon = attachment.look { sprite = CANNON, palette = 0, attach_point = 7 },
    hicannon = attachment.look { sprite = CANNON, palette = 1, attach_point = 7 },
    m_cannon = attachment.look { sprite = CANNON, palette = 2, attach_point = 7 },
}

local LOOK: ShooterLook = { sound = asset.sound("bn5:cannon"), attachment = attachment.kind, projectiles = projectile }

-- A chip's cannon.
function cannon.action(spec: CannonSpec): Action
    return common.action(spec, LOOK)
end

return cannon
