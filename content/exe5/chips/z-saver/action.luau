--!strict
-- Z Saver's action (EXE5's action 0x34, 0x080EE02A): EXE6's (@exelib/zsaver/action) in EXE5's look, with the Katanas'
-- step (lib/stepsword): a soul's charged Z Saver steps first and back after, leaving two afterimages on each swing's
-- sixth tick; and the command's window (B and back) shuts once A isn't held. Its blade is EXE5's sword (lib/swords),
-- its slashes' effects the effect table's rows 0x39 to 0x3B, its boom EXE5's sonic boom.

local attachment = require("../../objects/attachment")
local common = require("@exelib/zsaver/action")
local sonic_boom = require("@exelib/swords/sonic_boom")
local stepsword = require("../../lib/stepsword")
local swords = require("../../lib/swords")

local SLASH_SPRITE = asset.sprite("sword-slash")

return {
    action = common.make {
        swing_sound = asset.sound("z-saver"),
        command_sound = asset.sound("ok-8b"),
        attachment = attachment.kind,
        hold = nil,
        refreshes_form_overlay = true,
        recovery_clears_moving = true,
        blade = swords.blade,
        -- The held blade (lib/swords; 0x080EE192 and 0x080EE1FE call 0x080EC082).
        held_blade = swords.held_blade,
        blade_anim = swords.blade_anim,
        is = swords.slashes_navis.is,
        hits = swords.slashes_navis.hits,
        effects = {
            new.effect { sprite = SLASH_SPRITE, anim = 1, palette = 9 },
            new.effect { sprite = SLASH_SPRITE, anim = 0, palette = 9 },
            new.effect { sprite = SLASH_SPRITE, anim = 3, palette = 9 },
        },
        boom = function(owner: Object, x: number, y: number, element: number, damage: number, boom: SonicBoom): Object?
            return sonic_boom.throw(swords.sonic_boom, owner, x, y, element, damage, boom)
        end,
        step = stepsword,
        a_held_window = true,
    },
}
