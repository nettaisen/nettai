--!strict
-- InfVulc2 (BN5 chip 0x144): the Program Advance of three Vulcan2s with consecutive codes: action 0x19 with subtype
-- 5, the Vulcans' action with the InfVulcs' gun firing 24 of their bullets (objects/vulcan).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local vulcan = require("../../objects/vulcan/vulcan")
local recipe_vulcan2 = require("../vulcan2/chip")

return define.chip {
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
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 43, code_run = { chip = recipe_vulcan2, count = 3 } },
    },
    action = vulcan.infinite(24),
}
