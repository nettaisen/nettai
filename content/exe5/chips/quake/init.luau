--!strict
-- Quake1, Quake2 and Quake3: the quake series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bombs = require("../lib/bombs")
local quake = require("@self/quake")

-- Quake1 (EXE5 chip 0x023): a bomb lobbed three panels ahead that drops a weight there and, landing on solid ground,
-- cracks the panel and hits around it (level 0): EXE5's throw (lib/bombs, action 0x12 subtype 11) and its Quake bomb
-- (chips/quake/quake, attack object #0x51).
local quake1 = define.chip {
    id = "quake1",
    codes = { "A", "K", "P" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 100,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = bombs.held, thrower = quake.thrower(0) },
}

-- Quake2 (EXE5 chip 0x024): a bomb lobbed three panels ahead that drops a weight there and, landing on solid ground,
-- cracks the panel and hits around it (level 1): EXE5's throw (lib/bombs, action 0x12 subtype 11) and its Quake bomb
-- (chips/quake/quake, attack object #0x51).
local quake2 = define.chip {
    id = "quake2",
    codes = { "B", "L", "Y" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 1,
    mb = 25,
    damage = 150,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = bombs.held, thrower = quake.thrower(1) },
}

-- Quake3 (EXE5 chip 0x025): a bomb lobbed three panels ahead that drops a weight there and, landing on solid ground,
-- cracks the panel and hits around it (level 2): EXE5's throw (lib/bombs, action 0x12 subtype 11) and its Quake bomb
-- (chips/quake/quake, attack object #0x51).
local quake3 = define.chip {
    id = "quake3",
    codes = { "F", "H", "N" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 200,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = bombs.held, thrower = quake.thrower(2) },
}

return { quake1 = quake1, quake2 = quake2, quake3 = quake3 }
