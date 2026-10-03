--!strict
-- BN5's cannons' action (the original's action 0x14, BN6's code: @common/cannon) with BN5's attachment and
-- projectile; its sound and arms BN6's (their looks).

local attachment = require("../objects/attachment/attachment")
local common = require("@common/cannon")
local projectile = require("./projectile")

local cannon = {}

local CANNON = asset.sprite("bn6:cannon")
cannon.arms = {
    cannon = attachment.look { sprite = CANNON, palette = 0, attach_point = 7 },
    hicannon = attachment.look { sprite = CANNON, palette = 1, attach_point = 7 },
    m_cannon = attachment.look { sprite = CANNON, palette = 2, attach_point = 7 },
}

local LOOK: ShooterLook = { sound = asset.sound("bn6:cannon"), attachment = attachment.kind, projectiles = projectile }

-- A chip's cannon.
function cannon.action(spec: CannonSpec): Action
    return common.action(spec, LOOK)
end

return cannon
