--!strict
-- BN5's wide waves (BN6's chips/widesht/wave, the original's attack object #0x3B; BN5's the same code but for
-- its sprite, 10-1d where BN6's is 10-37), as BN5's chips need them: the kinds' speeds are BN6's
-- (`byte_80CB58C`, BN5's 0x080CC880: the same values), the collision types BN5's.

local collision = require("../../rules/collision")
local wave = require("@bn6/chips/widesht/wave")

local variants = {}

-- WideSht's (kind 3): it ends on its first hit.
variants.wide = wave.variant {
    speed = 0x6_0000,
    pierces = false,
    holds = false,
    trails = false,
    collision = collision.attack,
    sprite = asset.sprite("wide-wave"),
}

return variants
