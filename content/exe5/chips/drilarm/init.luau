--!strict
-- DrilArm1, DrilArm2 and DrilArm3: the drilarm series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local drill = require("@self/drill")

-- DrilArm1 (EXE5 chip 0x048): the navi's arm becomes a drill and two drills grind the two panels ahead for 24
-- ticks plus 10: EXE6's DrilArm (action 0x4F, EXE6's 0x2E) as EXE5's (chips/drilarm/drill).
local drilarm1: Chip = {
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
    action = drill.action { ticks = 24 },
    auto_battle = { class = "into_reach", range = "two_in_row" },
}

-- DrilArm2 (EXE5 chip 0x049): the navi's arm becomes a drill and two drills grind the two panels ahead for 24
-- ticks plus 10: EXE6's DrilArm (action 0x4F, EXE6's 0x2E) as EXE5's (chips/drilarm/drill).
local drilarm2: Chip = {
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
    action = drill.action { ticks = 24 },
    auto_battle = { class = "into_reach", range = "two_in_row" },
}

-- DrilArm3 (EXE5 chip 0x04a): the navi's arm becomes a drill and two drills grind the two panels ahead for 24
-- ticks plus 10: EXE6's DrilArm (action 0x4F, EXE6's 0x2E) as EXE5's (chips/drilarm/drill).
local drilarm3: Chip = {
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
    action = drill.action { ticks = 24 },
    auto_battle = { class = "into_reach", range = "two_in_row" },
}

return { drilarm1 = drilarm1, drilarm2 = drilarm2, drilarm3 = drilarm3 }
