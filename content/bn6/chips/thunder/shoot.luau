--!strict
-- Thunder's and DarkThnd's shot (the original's action 0x1F,
-- `sub_80EC7A6`): the shared shot (@common/thunder/shoot) with BN6's
-- buster and ball.

local ball = require("./ball")
local buster = require("../../lib/buster")
local common = require("@common/thunder/shoot")

local shoot = {}

-- One state for both chips' shots.
shoot.STATE = common.STATE

local LOOK: ThunderShotLook = { buster = buster, ball = ball.kind }

function shoot.action(spec: ThunderShot): Action
    return common.action(spec, LOOK)
end

return shoot
