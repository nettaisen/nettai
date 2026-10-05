--!strict
-- InfVulc1, InfVulc2 and InfVulc3: the infvulc series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local recipe_airspin1 = require("./airspin").airspin1
local recipe_airspin2 = require("./airspin").airspin2
local recipe_airspin3 = require("./airspin").airspin3
local recipe_vulcan1 = require("./vulcan").vulcan1
local recipe_vulcan2 = require("./vulcan").vulcan2
local recipe_vulcan3 = require("./vulcan").vulcan3
local vulcan = require("./vulcan/vulcan")

-- InfVulc1 (EXE5 chip 0x143): the Program Advance of three Vulcan1s with consecutive codes: action 0x19 with subtype
-- 4, the Vulcans' action with the InfVulcs' gun firing 20 of their bullets (chips/vulcan/vulcan).
local infvulc1 = define.chip {
    id = "infvulc1",
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
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 6, sequence = { recipe_airspin1, recipe_airspin1, recipe_vulcan1 }, operation_battle_only = true },
        { order = 42, code_run = { chip = recipe_vulcan1, count = 3 } },
    },
    action = vulcan.infinite(20),
}

-- InfVulc2 (EXE5 chip 0x144): the Program Advance of three Vulcan2s with consecutive codes: action 0x19 with subtype
-- 5, the Vulcans' action with the InfVulcs' gun firing 24 of their bullets (chips/vulcan/vulcan).
local infvulc2 = define.chip {
    id = "infvulc2",
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
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 7, sequence = { recipe_airspin2, recipe_airspin2, recipe_vulcan2 }, operation_battle_only = true },
        { order = 43, code_run = { chip = recipe_vulcan2, count = 3 } },
    },
    action = vulcan.infinite(24),
}

-- InfVulc3 (EXE5 chip 0x145): the Program Advance of three Vulcan3s with consecutive codes: action 0x19 with subtype
-- 6, the Vulcans' action with the InfVulcs' gun firing 28 of their bullets (chips/vulcan/vulcan).
local infvulc3 = define.chip {
    id = "infvulc3",
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
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 8, sequence = { recipe_airspin3, recipe_airspin3, recipe_vulcan3 }, operation_battle_only = true },
        { order = 44, code_run = { chip = recipe_vulcan3, count = 3 } },
    },
    action = vulcan.infinite(28),
}

return { infvulc1 = infvulc1, infvulc2 = infvulc2, infvulc3 = infvulc3 }
