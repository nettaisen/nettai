--!strict
-- BN5's wide wave (attack object #0x3B, BN6's code: @common/widesht/wave), BN5's kind: its sprite BN5's (a variant's
-- own, ./variants), its sound and afterimages' sprite BN6's, hitting navis; and BN5's wide shots
-- (@common/widesht/action, action 0x30) with BN5's attachment and wave: `wave.action { ... }`.

local action = require("@common/widesht/action")
local attachment = require("../attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@common/widesht/wave")

local wave = {}

wave.kind = common.make_kind {
    id = "bn5:widesht/wave",
    sprite = asset.sprite("bn5:wide-wave"),
    sound = asset.sound("bn6:wide-shot"),
    trail_sprite = asset.sprite("bn6:rock-cubes"),
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
