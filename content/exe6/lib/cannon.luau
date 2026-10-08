--!strict
-- EXE6's cannons' action (the original's action 0x14, `sub_80EBC0E`): the
-- shared action (@exelib/cannon) with EXE6's sound, attachment and
-- projectile. Each chip composes its own (`cannon.action { arm = ...,
-- shot = ... }`): what it holds (Cannon's, HiCannon's and M-Cannon's the
-- original's attachment rows 0 to 2 by `byte_80EBD2C`, at the navi's attach
-- point 7) and what it fires.

local attachment = require("../objects/attachment")
local common = require("@exelib/cannon")
local navi_arms = require("./navi_arms")
local projectile = require("./projectile")

local cannon = {}

-- One state for every cannon.
cannon.STATE = common.STATE

local LOOK: CannonLook = {
    sound = asset.sound("cannon"),
    -- `sub_80EBD68`: a player's arm by its AI index (`off_80EBD80`), anyone else's the plain one.
    hold = attachment.holder(navi_arms.cannon, true),
    projectiles = projectile,
    -- The navi raises the cannon in its animation 8 and lowers it in 7 (`sub_80EBC28`, `sub_80EBD3C`).
    raise_anim = 8,
    lower_anim = 7,
}

-- A chip's cannon.
function cannon.action(spec: CannonSpec): Action
    return common.action(spec, LOOK)
end

return cannon
