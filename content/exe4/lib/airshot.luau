--!strict
-- EXE4's AirShot, the chips' action 0x23 (0x080ECB74): @exelib/airshot (emap: its fire, 0x080ECB94, and its recovery,
-- 0x080ECC02, EXE5's, similar) with EXE4's look: the shot's sound (0x8D, a spreader's too); the navi in animation 12
-- (EXE5's and EXE6's 9) holds its air shooter in its related slot (+0x4C, EXE5's and EXE6's overlay), an attachment of
-- EXE4's own kind, its animation the navi's number; the projectiles EXE4's. A chip gives its shooter (attachment row
-- 9), its shot (a projectile row: the chip's parameter +0x10, AirShot's 4) and its recovery's count (0x080ECC0A: the
-- attack's variant, the store of 10 before it overwritten; 30 when it isn't 0).

local attachment = require("../objects/attachment")
local common = require("@exelib/airshot")
local projectile = require("./projectile")

local airshot = {}

-- The air shooter (attachment row 9, 0x080CD524: 0c-18 in palette 0).
airshot.shooter = attachment.look { sprite = asset.sprite("air-shooter"), palette = 0 }

-- The shooter's animation: NaviStats +0x23, the navi's number (MegaMan's 0).
local NAVI_NUMBER = 0

local LOOK: AirShotLook = {
    -- 0x080ECBC6.
    sound = asset.sound("spreader"),
    -- 0x080ECBB0: (navi << 8) | row, the attachment in +0x4C (0x080CD6E0).
    hold = function(me: Object, look: AttachmentLook, which: Slot)
        attachment.attach(me, look, which, NAVI_NUMBER)
    end,
    projectiles = projectile,
    anim = 12,
    slot = "related",
}

-- An AirShot: what the navi holds, what it fires and its recovery.
function airshot.action(spec: AirShotSpec): Action
    return common.action(spec, LOOK)
end

return airshot
