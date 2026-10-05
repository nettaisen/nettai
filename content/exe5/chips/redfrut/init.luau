--!strict
-- RedFrut1, RedFrut2 and RedFrut3: the redfrut series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local fruit = require("@self/fruit")

-- RedFrut1 (EXE5 chip 0x042): a hopping fruit whose breaker's side gets 300 HP back and 50 more mood (chips/redfrut/fruit,
-- instant effect 15; the record's params: gift 0, five hops).
local redfrut1 = define.chip {
    id = "redfrut1",
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

-- RedFrut2 (EXE5 chip 0x043): a hopping fruit whose breaker's side gets invulnerability for 420 ticks (chips/redfrut/fruit,
-- instant effect 15; the record's params: gift 1, five hops).
local redfrut2 = define.chip {
    id = "redfrut2",
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

-- RedFrut3 (EXE5 chip 0x044): a hopping fruit whose breaker's side gets Full Synchro (chips/redfrut/fruit,
-- instant effect 15; the record's params: gift 2, five hops).
local redfrut3 = define.chip {
    id = "redfrut3",
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

return { redfrut1 = redfrut1, redfrut2 = redfrut2, redfrut3 = redfrut3 }
