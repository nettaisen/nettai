--!strict
-- BN5's wide waves (the shared wave, @common/widesht/wave, BN6's attack object #0x3B; BN5's the same code but for
-- its sprite, 10-1d where BN6's is 10-37), as BN5's chips need them, for BN5's wave (./wave): the kinds' speeds are BN6's
-- (`byte_80CB58C`, BN5's 0x080CC880: the same values), the collision types BN5's. A chip names its kind in its
-- record's parameters (+0x10, the attack's +0x0C): WideSht1 to WideSht3 kinds 3 to 5, where BN6's one WideSht has
-- 3; SuprSpr1 to SuprSpr3 kinds 6 to 8, where BN6's one SuprSpr has 6.

local collision = require("../../rules/collision")
local wave = require("@common/widesht/wave")

local variants = {}

local SPRITE = asset.sprite("bn5:wide-wave")

-- A wave of `speed` (16.16 pixels a tick) that ends on its first hit, or goes through (`pierces`).
local function wide(speed: number, pierces: boolean?): WaveVariant
    return wave.variant {
        speed = speed,
        pierces = pierces or false,
        holds = false,
        trails = false,
        collision = collision.attack,
        sprite = SPRITE,
    }
end

-- WideSht1's (kind 3), WideSht2's (kind 4), WideSht3's (kind 5).
variants.wide = wide(0x6_0000)
variants.wide2 = wide(0x6_8000)
variants.wide3 = wide(0x7_0000)
-- SuprSpr1's (kind 6), SuprSpr2's (kind 7), SuprSpr3's (kind 8): they go through.
variants.super = wide(0x6_0000, true)
variants.super2 = wide(0x6_8000, true)
variants.super3 = wide(0x7_0000, true)

-- DarkWide's (kind 9, which BN6's table has and no BN6 chip fires): it goes through, trails afterimages of its
-- own sprite (animation 1: 0x080CC9A6's word 0x11D10, BN6's rock cubes 0x10010) and carries bug 0x14 with
-- argument 2, as BN5's row 0x25 in palette 1 (0x080CC82E).
variants.dark = wave.variant {
    speed = 0x7_0000,
    pierces = true,
    holds = false,
    trails = true,
    collision = collision.attack_480,
    bug = 0x214,
    palette = 1,
    sprite = SPRITE,
    trail_sprite = SPRITE,
}

return variants
