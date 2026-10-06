--!strict
-- JustcOne (EXE5 chip 0x0d8): a fist falls onto the other side's middle panel and cracks the panels around it: EXE6's
-- JustcOne (@exelib/justcone/strike, instant chip effect 20, EXE5's 0x080DBF54) with EXE5's look, the fist of no
-- element (EXE6's is Break), its landing's hits of the fist's.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local common = require("@exelib/justcone/strike")
local effects = require("../lib/effects")
local sparks = require("../lib/sparks")

-- The fist (attack object #0xAE, 0x080DBDA4).
local STRIKE = common.make {
    id = "justcone/strike",
    sprite = asset.sprite("justice-one"),
    sounds = { fall = asset.sound("justice-one"), land = asset.sound("hit-bomb-1") },
    is = collision.thrown_break,
    hits = collision.hits_navis,
    spark = sparks.breaking,
    wave_is = collision.attack,
    element = 0,
    waves_take_element = true,
    effects = { explosion = effects.explosion, burst = effects.burst, puff = effects.puff },
}

local chip: Chip = {
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

return { justcone = chip }
