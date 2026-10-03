--!strict
-- RedFrut1 (BN5 chip 0x042): a hopping fruit whose breaker's side gets 300 HP back and 50 more mood (chips/redfrut/fruit,
-- instant effect 15; the record's params: gift 0, five hops).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local fruit = require("../redfrut/fruit")

return define.chip {
    id = "bn5:redfrut1",
    codes = { "E", "K", "Z" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 24,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    instant = fruit.instant(0, 5),
}
