--!strict
-- BassAnly, the navi of BN5's BassAnly chip (BN5's actor object #0x50, 0x080C2C9C): the shared BassAnly
-- (@exelib/bassanly/navi), BN5's kind: Bass's sprite 08-13, his cloak on his animation + 0x10 (0x080C2D40:
-- 0x10011308), BN5's animations (9 his arm rising, 10 up, 11 firing, 12 its loop, 13 after), his arm while he takes
-- aim (BN5's attachment row 0x12, Bass's sprite, its animation 0x24: 0x080C2DC8), and BN5's dark ball (./shot).

local attachment = require("../../objects/attachment/attachment")
local common = require("@exelib/bassanly/navi")
local effects = require("../../lib/effects")
local shot = require("./shot")

local SPRITE = asset.sprite("bn5:bass")

return common.make {
    id = "bn5:bassanly/navi",
    sprite = SPRITE,
    cloak = { sprite = SPRITE, stepping = "while_dimmed", anim_offset = 0x10 },
    sounds = { appear = asset.sound("bn5:appear"), raise = asset.sound("bn5:bass-anly") },
    anims = { raise = 9, stance = 0xA, fire = 0xB, fire_loop = 0xC, rest = 0xD },
    arm = { attach = attachment.attach, look = attachment.look { sprite = SPRITE, palette = 0 }, anim = 0x24 },
    smoke = effects.smoke,
    -- His shots (0x080C2E94): the fastest, seeking enemy navis along the slant.
    shoot = function(owner: Object, x: number, y: number, slant: number, damage: number): Object?
        return shot.spawn(owner, x, y, 0, { level = 3, slant = slant, homing = "seeking" }, damage, 0)
    end,
}
