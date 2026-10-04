--!strict
-- JustcOne (BN5 chip 0x0d8): a fist falls onto the other side's middle panel and cracks the panels around it: BN6's
-- JustcOne (@common/justcone/strike, instant chip effect 20, BN5's 0x080DBF54) with BN5's look, the fist of no
-- element (BN6's is Break), its landing's hits of the fist's.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local collision = require("../../rules/collision")
local common = require("@common/justcone/strike")
local effects = require("../../lib/effects")
local sparks = require("../../lib/sparks")

-- The fist (attack object #0xAE, 0x080DBDA4).
local STRIKE = common.make {
    id = "bn5:justcone/strike",
    sprite = asset.sprite("bn5:justice-one"),
    sounds = { fall = asset.sound("bn5:justice-one"), land = asset.sound("bn5:hit-bomb-1") },
    is = collision.thrown_break,
    hits = collision.hits_navis,
    spark = sparks.breaking,
    wave_is = collision.attack,
    element = 0,
    waves_take_element = true,
    effects = { explosion = effects.explosion, burst = effects.burst, puff = effects.puff },
}

return define.chip {
    id = "bn5:justcone",
    codes = { "J" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 4,
    mb = 90,
    damage = 280,
    hit_param = 178,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    megaman = "light",
    instant = STRIKE.instant,
}
