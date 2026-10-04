--!strict
-- Guard1, Guard2 and Guard3: the guard series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local guard = require("../lib/guard")

-- Guard1 (BN5 chip 0x058): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- shock wave back along the row with the chip's damage: BN5's guard (lib/guard, action 0x1F with subtype 0).
local guard1 = define.chip {
    id = "guard1",
    codes = { "A", "D", "K", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 50,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = guard.looks.guard1, counter = { kind = "wave" } },
}

-- Guard2 (BN5 chip 0x059): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- shock wave back along the row with the chip's damage: BN5's guard (lib/guard, action 0x1F with subtype 1).
local guard2 = define.chip {
    id = "guard2",
    codes = { "B", "L", "Q", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 10,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = guard.looks.guard2, counter = { kind = "wave" } },
}

-- Guard3 (BN5 chip 0x05a): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- shock wave back along the row with the chip's damage: BN5's guard (lib/guard, action 0x1F with subtype 2).
local guard3 = define.chip {
    id = "guard3",
    codes = { "C", "M", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 16,
    damage = 150,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = guard.looks.guard3, counter = { kind = "wave" } },
}

return { guard1 = guard1, guard2 = guard2, guard3 = guard3 }
