--!strict
-- Z Saver's action (the original's action 0x5B, `sub_80EFEE0`;
-- docs/engine/shot-chips.md §13): the one BN5 shares (@common/zsaver/action)
-- in BN6's look. Three slashes with growing reach; B and Left together in
-- the twelve ticks after the third slash's hit give a fourth, a piercing
-- sonic boom. (The original's weapon routine 0x6E starts the same action.)

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@common/zsaver/action")
local parts = require("../../lib/swords/parts")
local sonic_boom = require("../../lib/swords/sonic_boom")

-- The slashes' effects over the panel ahead (effects 0x65 to 0x67,
-- `dword_80F0064`).
local SLASH_SPRITE = asset.sprite("bn6:sword-slash")

return {
    action = common.make {
        id = "bn6:z-saver/action",
        swing_sound = asset.sound("bn6:z-saver"),
        command_sound = asset.sound("bn6:ok-8b"),
        attachment = attachment.kind,
        blade = parts.blades.sword,
        blade_anim = parts.blade_anim,
        is = collision.slash,
        hits = collision.hits_navis,
        effects = {
            define.effect { sprite = SLASH_SPRITE, anim = 1, palette = 4 },
            define.effect { sprite = SLASH_SPRITE, anim = 0, palette = 4 },
            define.effect { sprite = SLASH_SPRITE, anim = 3, palette = 4 },
        },
        boom = function(owner: Object, x: number, y: number, element: number, damage: number, boom: SonicBoom): Object?
            return sonic_boom.throw(owner, x, y, element, damage, boom)
        end,
        step = nil,
        a_held_window = false,
    },
}
