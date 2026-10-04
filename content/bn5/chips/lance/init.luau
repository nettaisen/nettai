--!strict
-- Lance (BN5 chip 0x051): three lances thrust out of the far column's panels, one per row: BN6's Lance (instant
-- chip effect 4) with BN5's lance (chips/lance/lance).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bn5_lance = require("@self/lance")
local lance = require("@exelib/lance")

return define.chip {
    id = "lance",
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
    instant = lance.instant_of(bn5_lance.kind, 0),
}
