--!strict
-- RedFrut3 (BN5 chip 0x044): a hopping fruit whose breaker's side gets Full Synchro (chips/redfrut/fruit,
-- instant effect 15; the record's params: gift 2, five hops).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local fruit = require("../redfrut/fruit")

return define.chip {
    id = "bn5:redfrut3",
    codes = { "H", "O", "U" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 62,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    instant = fruit.instant(2, 5),
}
