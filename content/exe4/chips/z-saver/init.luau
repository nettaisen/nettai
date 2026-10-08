--!strict
-- Z Saver (EXE4 chip 0x0DA; its record at 0x080197EC + 0xDA * 0x2C): three slashes with growing reach (the panel ahead
-- and the next, the column, the three ahead: the region words at 0x080EE908), a fourth (a piercing sonic boom of hit
-- modifier 3) for B and Left in the twelve ticks after the third's hit, A held: action 0x70 (0x080EE774), the shared
-- one (@exelib/zsaver/action, which EXE5's grew from) in EXE4's look: the blade held in the related slot as EXE4's own
-- attachment (ProtoSoul's 13 or the navi's number: lib/swords), let go of at each swing's end; no form overlay
-- refreshed, nor the moving flag cleared; the swing's sound 0x15F, the command's 0x10B; the slashes' effects the
-- effect table's rows 0x65 to 0x67; the boom EXE4's sonic boom. (A navi whose NaviStats +0x26 is 2, only the story's
-- other navis, skips the command: 0x080EE7C6.)
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local common = require("@exelib/zsaver/action")
local sonic_boom = require("@exelib/swords/sonic_boom")
local sonic_boom_kind = require("../objects/sonic_boom").kind
local swords = require("../lib/swords")

local SLASH_SPRITE = asset.sprite("sword-slash")

local zsaver: Chip = {
    action = common.make {
        id = "z-saver/action",
        swing_sound = asset.sound("z-saver"),
        command_sound = asset.sound("sword-command"),
        attachment = nil,
        hold = function(me: Object, blade: AttachmentLook, anim: number)
            attachment.attach(me, blade, "related", anim)
        end,
        refreshes_form_overlay = false,
        recovery_clears_moving = false,
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        is = collision.slash,
        hits = collision.hits_navis,
        effects = {
            new.effect { sprite = SLASH_SPRITE, anim = 1, palette = 9 },
            new.effect { sprite = SLASH_SPRITE, anim = 0, palette = 9 },
            new.effect { sprite = SLASH_SPRITE, anim = 3, palette = 9 },
        },
        boom = function(owner: Object, x: number, y: number, element: number, damage: number, boom: SonicBoom): Object?
            return sonic_boom.throw(sonic_boom_kind, owner, x, y, element, damage, boom)
        end,
        step = nil,
        a_held_window = true,
    },
    codes = { "Z" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 80,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
}

return { ["z-saver"] = zsaver }
