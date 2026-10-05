--!strict
-- EXE5's wide wave (attack object #0x3B, EXE6's code: @exelib/widesht/wave), EXE5's kind: its sprite EXE5's (a variant's
-- own, ./variants), its sound EXE6's number, hitting navis (its afterimages copy its sheet; EXE5's one trailing variant
-- names its own); and EXE5's wide shots
-- (@exelib/widesht/action, action 0x30) with EXE5's attachment and wave: `wave.action { ... }`.

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

-- EXE5's wide shooter (EXE6's row 0x23's look, EXE5's sprite).
local SHOOTER = attachment.look { sprite = asset.sprite("wide-shooter"), palette = 0, attach_point = 12 }

local LOOK: WideShotLook = { shooter = SHOOTER, attachment = attachment.kind, wave = wave.kind }

-- A wide shot.
function wave.action(spec: WideShotSpec): Action
    return action.action(spec, LOOK)
end

return wave
