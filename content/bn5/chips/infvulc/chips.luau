--!strict
-- InfVulc1, InfVulc2 and InfVulc3: the infvulc series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local vulcan = require("../vulcan/vulcan")

-- InfVulc1 (BN5 chip 0x143): the Program Advance of three Vulcan1s with consecutive codes: action 0x19 with subtype
-- 4, the Vulcans' action with the InfVulcs' gun firing 20 of their bullets (chips/vulcan/vulcan).
local infvulc1 = define.chip {
    id = "bn5:infvulc1",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 20,
    hit_param = 2,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = vulcan.infinite(20),
}

-- InfVulc2 (BN5 chip 0x144): the Program Advance of three Vulcan2s with consecutive codes: action 0x19 with subtype
-- 5, the Vulcans' action with the InfVulcs' gun firing 24 of their bullets (chips/vulcan/vulcan).
local infvulc2 = define.chip {
    id = "bn5:infvulc2",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 20,
    hit_param = 2,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = vulcan.infinite(24),
}

-- InfVulc3 (BN5 chip 0x145): the Program Advance of three Vulcan3s with consecutive codes: action 0x19 with subtype
-- 6, the Vulcans' action with the InfVulcs' gun firing 28 of their bullets (chips/vulcan/vulcan).
local infvulc3 = define.chip {
    id = "bn5:infvulc3",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 20,
    hit_param = 2,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = vulcan.infinite(28),
}

return { infvulc1 = infvulc1, infvulc2 = infvulc2, infvulc3 = infvulc3 }
