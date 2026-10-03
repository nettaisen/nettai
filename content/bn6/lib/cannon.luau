--!strict
-- BN6's cannons' action (the original's action 0x14, `sub_80EBC0E`): the
-- shared action (@common/cannon) with BN6's sound, attachment and
-- projectile. Each chip composes its own (`cannon.action { arm = ...,
-- shot = ... }`): what it holds and what it fires.

local attachment = require("../objects/attachment/attachment")
local common = require("@common/cannon")
local projectile = require("./projectile")

local cannon = {}

-- One state for every cannon.
cannon.STATE = common.STATE

-- The cannons' looks: the arm the navi holds (the original's attachment
-- rows 0 to 2, by `byte_80EBD2C`), at its attach point 7.
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
