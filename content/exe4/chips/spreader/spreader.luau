--!strict
-- EXE4's spreaders (the chips' action 0x1E, 0x080EC760): EXE6's (@exelib/spreadr/action) with EXE4's look: its sound
-- (0x8D), its gun held in the related slot as EXE4's own attachment (its animation twice the navi's number, NaviStats
-- +0x23, MegaMan's 0), its bullets (objects/bullet), and the navi's animation 14 as it fires. Each chip composes its
-- own from its variant's rows: the gun (0x080EC814, an attachment row: Spreader's 0x0A, HeatShot's 6, Bubbler's 0x0E,
-- DrkSpred's 0x32), the delay (0x080EC818: 1, 10, 10, 1), the shot's height (0x080EC81C: 0x10, 0x10, 0, 0x10) and the
-- bullet's variant (its parameter +0x10).

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local common = require("@exelib/spreadr/action")

local spreader = {}

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local GUN = asset.sprite("spreader")
-- The guns (attachment rows 0x0A and 0x32: 0c-19 in palettes 0 and 3; row 6: 0c-0d; row 0x0E: 0c-1c).
spreader.guns = {
    spreader = attachment.look { sprite = GUN, palette = 0 },
    heatshot = attachment.look { sprite = asset.sprite("heatshot"), palette = 0 },
    bubbler = attachment.look { sprite = asset.sprite("bubbler"), palette = 0 },
    dark = attachment.look { sprite = GUN, palette = 3 },
}

local LOOK: SpreaderLook = {
    sound = asset.sound("spreader"),
    -- 0x080EC796: EXE4's own attachment in the related slot, its animation twice the navi's number.
    hold = function(me: Object, look: AttachmentLook, which: Slot)
        attachment.attach(me, look, which, 2 * NAVI_NUMBER)
    end,
    slot = "related",
    bullet_kind = bullet.kind,
    -- The navi fires in its animation 14 (0x080EC788).
    anim = 14,
}

-- A chip's spreader: its key, gun, the count past which it fires, the shot's height in pixels and its bullet.
function spreader.action(spec: { id: string, gun: AttachmentLook, delay: number, z: number, shot: BulletVariant }): Action
    return common.action({
        id = spec.id,
        gun = spec.gun,
        fire_after = spec.delay,
        z = spec.z * 0x10000,
        bullet = spec.shot,
    }, LOOK)
end

return spreader
