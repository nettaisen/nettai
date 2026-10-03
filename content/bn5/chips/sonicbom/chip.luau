--!strict
-- SonicBom (BN5 chip 0x173): a swing that throws a sonic boom down the row (VarSwrd's pick: no library chip): BN5's
-- action 0x2B (action.luau: BN6's SonicBom swing with the Katanas' step) and BN5's sonic boom (lib/swords, attack
-- object #0x58), hit modifier 3.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local sonicbom = require("./action")

return define.chip {
    id = "bn5:sonicbom",
    codes = { "E", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "special",
    rarity = 0,
    mb = 8,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = sonicbom.action,
}
