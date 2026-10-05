--!strict
-- Thunder's and DarkThnd's shot (the original's action 0x1F,
-- `sub_80EC7A6`): the shared shot (@exelib/thunder/shoot) with EXE6's
-- buster and ball.

local ball = require("./ball")
local buster = require("../../lib/buster")
local common = require("@exelib/thunder/shoot")

local shoot = {}

-- One state for both chips' shots.
shoot.STATE = common.STATE

local LOOK: ThunderShotLook = { buster = buster, ball = ball.kind }

function shoot.action(spec: ThunderShot): Action
    return common.action(spec, LOOK)
end

return shoot
