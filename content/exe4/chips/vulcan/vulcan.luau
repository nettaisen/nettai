--!strict
-- EXE4's vulcans (the chips' action 0x1F, 0x080EC842): EXE6's and EXE5's (@exelib/vulcan/action) with EXE4's look:
-- its sound (0xBB), its gun held in the related slot as EXE4's own attachment (its animation three times the navi's
-- number, NaviStats +0x23, MegaMan's 0), its bullets (objects/bullet), and the navi's animations 13 as it holds the
-- gun and 17 as it fires. Each chip composes its own from its variant's rows: its gun (attachment row 13, DrkVulcn's
-- 49: the gun sprite 0c-1d in palette 0 or 3), the shots (0x080EC934: 3, 5, 7, 12, 24) and the bullet's variant (its
-- parameter +0x10).

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local common = require("@exelib/vulcan/action")

local vulcan = {}

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local LOOK: VulcanLook = {
    sound = asset.sound("vulcan"),
    -- 0x080EC874: EXE4's own attachment in the related slot, its animation three times the navi's number.
    hold = function(me: Object, look: AttachmentLook, which: Slot)
        attachment.attach(me, look, which, 3 * NAVI_NUMBER)
    end,
    slot = "related",
    bullet_kind = bullet.kind,
    -- The navi holds the gun in its animation 13 (0x080EC868) and fires in 17 (0x080EC8C8).
    hold_anim = 13,
    fire_anim = 17,
}

-- A chip's vulcan: its key, gun, shots and bullet.
function vulcan.action(spec: { id: string, gun: AttachmentLook, shots: number, shot: BulletVariant }): Action
    return common.action({ id = spec.id, arm = spec.gun, shots = spec.shots, bullet = spec.shot }, LOOK)
end

return vulcan
