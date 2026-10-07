--!strict
-- EXE5's wide wave (attack object #0x3B, EXE6's code: @exelib/widesht/wave), EXE5's kind: its sprite EXE5's (a variant's
-- own), its sound EXE6's number, hitting navis (its afterimages copy its sheet; EXE5's one trailing variant names its
-- own); and EXE5's wide shots (@exelib/widesht/action, action 0x30) with EXE5's attachment and wave:
-- `wave.action { ... }`. Each chip states its wave: a kind of its record's parameters (+0x10, the attack's +0x0C):
-- WideSht1 to WideSht3 kinds 3 to 5, where EXE6's one WideSht has 3; SuprSpr1 to SuprSpr3 kinds 6 to 8, where EXE6's
-- one SuprSpr has 6; DarkWide's 9. The kinds' speeds are EXE6's (`byte_80CB58C`, EXE5's 0x080CC880: the same values),
-- the collision types EXE5's.

local action = require("@exelib/widesht/action")
local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/widesht/wave")

local wave = {}

wave.kind = common.make_kind {
    id = "widesht/wave",
    sprite = asset.sprite("wide-wave"),
    sound = asset.sound("wide-shot"),
    trail_sprite = asset.sprite("wide-wave"),
    target = collision.hits_navis,
}

-- EXE5's wide shooter (EXE6's row 0x23's look, EXE5's sprite: its attachment row 0x11, sheet 0c-2a in palette 0 on
-- point 12). (DarkWide holds its own.)
local SHOOTER = attachment.look { sprite = asset.sprite("wide-shooter"), palette = 0, attach_point = 12 }

local SPRITE = asset.sprite("wide-wave")

-- A wave variant.
wave.variant = common.variant

-- A plain wide wave (a kind of 3 to 8) of `speed` (16.16 pixels a tick) that ends on its first hit, or goes through
-- (`pierces`).
function wave.wide(speed: number, pierces: boolean): WaveVariant
    return common.variant {
        speed = speed,
        pierces = pierces,
        holds = false,
        trails = false,
        collision = collision.attack,
        sprite = SPRITE,
    }
end

local LOOK: WideShotLook = { shooter = SHOOTER, attachment = attachment.kind, wave = wave.kind }

-- A wide shot.
function wave.action(spec: WideShotSpec): Action
    return action.action(spec, LOOK)
end

return wave
