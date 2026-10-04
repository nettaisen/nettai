--!strict
-- BN5's wide wave (attack object #0x3B, BN6's code: @exelib/widesht/wave), BN5's kind: its sprite BN5's (a variant's
-- own, ./variants), its sound BN6's number, hitting navis (its afterimages copy its sheet; BN5's one trailing variant
-- names its own); and BN5's wide shots
-- (@exelib/widesht/action, action 0x30) with BN5's attachment and wave: `wave.action { ... }`.

local action = require("@exelib/widesht/action")
local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/widesht/wave")

local wave = {}

wave.kind = common.make_kind {
    id = "bn5:widesht/wave",
    sprite = asset.sprite("bn5:wide-wave"),
    sound = asset.sound("bn5:wide-shot"),
    trail_sprite = asset.sprite("bn5:wide-wave"),
    target = collision.hits_navis,
}

-- BN5's wide shooter (BN6's row 0x23's look, BN5's sprite).
local SHOOTER = attachment.look { sprite = asset.sprite("bn5:wide-shooter"), palette = 0, attach_point = 12 }

local LOOK: WideShotLook = { shooter = SHOOTER, attachment = attachment.kind, wave = wave.kind }

-- A wide shot.
function wave.action(spec: WideShotSpec): Action
    return action.action(spec, LOOK)
end

return wave
