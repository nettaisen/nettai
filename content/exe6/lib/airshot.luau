--!strict
-- EXE6's AirShot's action (the original's action 0x21, `sub_80EC884`): the
-- shared action (@exelib/airshot) with EXE6's sound, attachment and
-- projectile. A chip gives what the navi holds and what it fires
-- (`airshot.action { shooter = ..., shot = ... }`).

local attachment = require("../objects/attachment")
local common = require("@exelib/airshot")
local navi_arms = require("./navi_arms")
local projectile = require("./projectile")

local airshot = {}

-- EXE6's air shooter (the original's attachment row 0x13).
airshot.shooter = attachment.look { sprite = asset.sprite("air-shooter"), palette = 0, attach_point = 10 }

local LOOK: AirShotLook = {
    sound = asset.sound("h-burst"),
    -- `sub_80EC934`: the shooter by the holder's AI index (`off_80EC944`).
    hold = attachment.holder(navi_arms.air_shooter),
    projectiles = projectile,
    anim = 9,
    slot = "overlay",
}

-- An AirShot: what the navi holds and what it fires.
function airshot.action(spec: AirShotSpec): Action
    return common.action(spec, LOOK)
end

return airshot
