--!strict
-- BN5's throw (action 0x12) and thrown bomb (attack object #8): BN6's code (@common/bombs), BN5's attachment and
-- bomb kind; the throw's sound, the bomb's sprite, sound and effects BN6's. A chip's throw is
-- `bombs.throw { held = ..., thrower = ... }`; MiniBomb's, EnergBom's and MegEnBom's thrower `bombs.thrower(variant)`.

local attachment = require("../objects/attachment/attachment")
local bomb = require("@common/bombs/bomb")
local effects = require("@bn6/lib/effects")
local throw = require("@common/bombs/throw")

local bombs = {}

local SPRITE = asset.sprite("bn6:bomb")

-- What the navi holds while throwing a bomb.
bombs.held = attachment.look { sprite = SPRITE, palette = 0 }

-- A bomb variant.
bombs.variant = bomb.variant

local made = bomb.make {
    id = "bn5:bomb",
    sprite = SPRITE,
    hit_sound = asset.sound("bn6:hit-bomb-1"),
    splash = effects.splash,
    explosion = effects.explosion,
    puff = effects.puff,
}

-- A thrower that throws a bomb of `variant`, null-element.
bombs.thrower = made.thrower

local THROW: ThrowLook = { sound = asset.sound("bn6:minibomb-throw"), attachment = attachment.kind }

-- A chip's throw.
function bombs.throw(spec: ThrowSpec): Action
    return throw.action(spec, THROW)
end

return bombs
