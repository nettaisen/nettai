--!strict
-- BN5's projectile variants (the original's `off_80C4C78` rows), as BN5's chips need them: for BN5's projectile
-- (./projectile, BN6's code) with BN5's collision types (rules/collision: BN6's rows without the 0x80 self bit,
-- docs/design/bn5-map.md §15.1).

local collision = require("../../rules/collision")
local projectile = require("./projectile")
local sparks = require("../../lib/sparks")

local variants = {}

local SHOT = { is = collision.attack, hits = collision.hits_navis }

-- The buster's plain shot (row 0).
variants.plain = projectile.variant { id = "bn5:shot/plain", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain }
-- A plain shot with the charged shot's spark (row 3): the computer navi's buster's (rules/computer-navi/buster).
variants.bright = projectile.variant { id = "bn5:shot/bright", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged }
-- The charged shot (row 6).
variants.charged = projectile.variant { id = "bn5:shot/charged", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged }
-- The cannons' shot (row 1): it staggers (hit modifier 3).
variants.cannon = projectile.variant { id = "bn5:shot/cannon", collision = SHOT, hit_mod = 3, element = "null", spark = sparks.impact }

-- AirShot's shot (BN5's row 4): it pushes (hit modifier 0x61), without BN6's wind element (BN5 has none).
variants.air = projectile.variant {
    id = "bn5:shot/air",
    collision = { is = collision.pushing, hits = collision.hits_navis },
    hit_mod = 0x61,
    element = "null",
    spark = sparks.impact,
}

return variants
