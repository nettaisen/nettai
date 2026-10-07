--!strict
-- EXE6's cannons' action (the original's action 0x14, `sub_80EBC0E`): the
-- shared action (@exelib/cannon) with EXE6's sound, attachment and
-- projectile. Each chip composes its own (`cannon.action { arm = ...,
-- shot = ... }`): what it holds and what it fires.

local attachment = require("../objects/attachment")
local common = require("@exelib/cannon")
local navi_arms = require("./navi_arms")
local projectile = require("./projectile")

local cannon = {}

-- One state for every cannon.
cannon.STATE = common.STATE

-- The cannons' looks: the arm the navi holds (the original's attachment
-- rows 0 to 2, by `byte_80EBD2C`), at its attach point 7.
local CANNON = asset.sprite("cannon")
cannon.arms = {
    cannon = attachment.look { sprite = CANNON, palette = 0, attach_point = 7 },
    hicannon = attachment.look { sprite = CANNON, palette = 1, attach_point = 7 },
    m_cannon = attachment.look { sprite = CANNON, palette = 2, attach_point = 7 },
}

local LOOK: ShooterLook = {
    sound = asset.sound("cannon"),
    -- `sub_80EBD68`: a player's arm by its AI index (`off_80EBD80`), anyone else's the plain one.
    hold = attachment.holder(navi_arms.cannon, true),
    projectiles = projectile,
}

-- A chip's cannon.
function cannon.action(spec: CannonSpec): Action
    return common.action(spec, LOOK)
end

return cannon
