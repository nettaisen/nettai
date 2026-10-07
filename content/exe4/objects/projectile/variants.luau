--!strict
-- EXE4's projectile variants (0x080CD2A8's rows), as EXE4's actions fire them: what each is and hits (EXE4's
-- collision rows, rules/collision), its hit modifier, element and spark (lib/sparks); no sprite.

local collision = require("../../rules/collision")
local projectile = require("../projectile")
local sparks = require("../../lib/sparks")

local variants = {}

-- Row 0: the buster's shot (action 8, 0x080EB380, its attack parameter's 0).
variants.plain = projectile.variant {
    id = "shot/plain",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.plain,
}

-- Row 1: the cannon's shot (action 0x0B's variants 0 to 3, 0x080EB97A): it staggers (hit modifier 3), with the
-- impact spark.
variants.cannon = projectile.variant {
    id = "shot/cannon",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.impact,
}

-- Row 6: the charged shot (action 0x24, 0x080ECC7E).
variants.charged = projectile.variant {
    id = "shot/charged",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.charged,
}

return variants
