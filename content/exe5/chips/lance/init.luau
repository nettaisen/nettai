--!strict
-- Lance (EXE5 chip 0x051): three lances thrust out of the far column's panels, one per row: EXE6's Lance (instant
-- chip effect 4) with EXE5's lance (chips/lance/lance).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local exe5_lance = require("@self/lance")
local lance = require("@exelib/lance")
local sparks = require("../lib/sparks")

-- Its hit (0x080CF904's row 0): an attack against attacks of hit modifier 0x10 (EXE6's 0x51) with the wood spark.
local HIT = lance.hit { is = collision.attack, hits = collision.attack, spark = sparks.wood, hit_mod = 0x10 }

local chip: Chip = {
    codes = { "C", "L", "R", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 3,
    mb = 42,
    damage = 130,
    hit_param = 30,
    lockout = 20,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    instant = lance.instant_of(exe5_lance.kind, 0, HIT),
    auto_battle = { class = "at_once" },
}

return { lance = chip }
