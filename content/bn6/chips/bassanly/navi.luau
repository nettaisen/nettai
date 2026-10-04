--!strict
-- BassAnly, the navi of the BassAnly chip (the original's actor object
-- #0x50, `sub_80C3CE8`): the shared BassAnly (@exelib/bassanly/navi), BN6's
-- kind. He appears (with his cloak, a form overlay) on his user's panel,
-- waits 51 ticks, raises his arm, and fires four dark balls in turn, up and
-- down the slant, that home in on enemy navis (./shot). Then he goes in a
-- burst, and the controller moves on. He goes at once if his panel is off
-- the field. See docs/engine/chips.md §3.6.35.

local common = require("@exelib/bassanly/navi")
local effects = require("../../lib/effects")
local shot = require("./shot")

local SPRITE = asset.sprite("bass-anly")

return common.make {
    id = "bassanly/navi",
    sprite = SPRITE,
    -- His cloak: a form overlay on his sprite, stepping while dimmed, on his
    -- animation + 0x14 (`dword_80C3D98`).
    cloak = { sprite = SPRITE, stepping = "while_dimmed", anim_offset = 0x14 },
    sounds = { appear = asset.sound("appear"), raise = asset.sound("bass-anly") },
    anims = { raise = 8, stance = 0xD, fire = 9, fire_loop = nil, rest = 0 },
    arm = nil,
    smoke = effects.smoke,
    -- His shots: the fastest, seeking enemy navis along the slant.
    shoot = function(owner: Object, x: number, y: number, slant: number, damage: number): Object?
        return shot.spawn(owner, x, y, 0, { level = 3, slant = slant, homing = "seeking" }, damage, 0)
    end,
}
