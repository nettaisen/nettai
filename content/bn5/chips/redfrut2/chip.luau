--!strict
-- RedFrut2 (BN5 chip 0x043): a hopping fruit whose breaker's side gets invulnerability for 420 ticks (chips/redfrut/fruit,
-- instant effect 15; the record's params: gift 1, five hops).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local fruit = require("../redfrut/fruit")

return define.chip {
    id = "bn5:redfrut2",
    codes = { "F", "T", "W" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 1,
    mb = 48,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    instant = fruit.instant(1, 5),
}
