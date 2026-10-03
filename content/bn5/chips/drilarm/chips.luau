--!strict
-- DrilArm1, DrilArm2 and DrilArm3: the drilarm series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local drill = require("./drill")

-- DrilArm1 (BN5 chip 0x048): the navi's arm becomes a drill and two drills grind the two panels ahead for 24
-- ticks plus 10: BN6's DrilArm (action 0x4F, BN6's 0x2E) as BN5's (chips/drilarm/drill).
local drilarm1 = define.chip {
    id = "bn5:drilarm1",
    codes = { "A", "J", "W" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 0,
    mb = 38,
    damage = 60,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = drill.action(false),
}

-- DrilArm2 (BN5 chip 0x049): the navi's arm becomes a drill and two drills grind the two panels ahead for 24
-- ticks plus 10: BN6's DrilArm (action 0x4F, BN6's 0x2E) as BN5's (chips/drilarm/drill).
local drilarm2 = define.chip {
    id = "bn5:drilarm2",
    codes = { "D", "L", "S" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 1,
    mb = 51,
    damage = 70,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = drill.action(false),
}

-- DrilArm3 (BN5 chip 0x04a): the navi's arm becomes a drill and two drills grind the two panels ahead for 24
-- ticks plus 10: BN6's DrilArm (action 0x4F, BN6's 0x2E) as BN5's (chips/drilarm/drill).
local drilarm3 = define.chip {
    id = "bn5:drilarm3",
    codes = { "I", "P", "Z" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 64,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = drill.action(false),
}

return { drilarm1 = drilarm1, drilarm2 = drilarm2, drilarm3 = drilarm3 }
