--!strict
-- BN5's projectile variants (BN6's objects/projectile, the original's `off_80C4C78` rows), as BN5's chips need
-- them: BN6's projectile (@bn6/objects/projectile; BN5's the same code) with BN5's collision types (rules/collision:
-- BN6's rows without the 0x80 self bit, docs/design/bn5-map.md §15.1).

local collision = require("../../rules/collision")
local projectile = require("@bn6/objects/projectile/projectile")
local sparks = require("@bn6/lib/sparks")

local variants = {}

local SHOT = { is = collision.attack, hits = collision.hits_navis }

-- The cannons' shot: it staggers (hit modifier 3).
variants.cannon = projectile.variant { id = "shot/cannon", collision = SHOT, hit_mod = 3, element = "null", spark = sparks.impact }

-- AirShot's shot (BN5's row 4): it pushes (hit modifier 0x61), without BN6's wind element (BN5 has none).
variants.air = projectile.variant {
    id = "shot/air",
    collision = { is = collision.pushing, hits = collision.hits_navis },
    hit_mod = 0x61,
    element = "null",
    spark = sparks.impact,
}

return variants
