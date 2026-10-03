--!strict
-- BN6's thrown bomb (the original's attack object #8, `sub_80C5BB0`): the
-- shared bomb (@common/bombs/bomb), BN6's kind and look. MiniBomb, EnergBom,
-- MegEnBom and BigBomb: what differs between the chips is its variant, a
-- record (`bomb.variant { ... }`).

local attachment = require("../../objects/attachment/attachment")
local common = require("@common/bombs/bomb")
local effects = require("../effects")

local bomb = {}

local SPRITE = asset.sprite("bn6:bomb")

-- What the navi holds while throwing a bomb (the original's attachment row
-- 0x04; BigBomb's throw adds 3 to its palette).
bomb.held = attachment.look { sprite = SPRITE, palette = 0 }

-- A bomb variant.
bomb.variant = common.variant

local made = common.make {
    id = "bn6:bomb",
    sprite = SPRITE,
    hit_sound = asset.sound("bn6:hit-bomb-1"),
    splash = effects.splash,
    explosion = effects.explosion,
    puff = effects.puff,
}
bomb.kind = made.kind

-- `sub_80C5DBC`: a thrower for `throw.action` that throws a bomb of
-- `variant`, null-element.
bomb.thrower = made.thrower

return bomb
