--!strict
-- InfVulc1 (BN5 chip 0x143): the Program Advance of three Vulcan1s with consecutive codes: action 0x19 with subtype
-- 4, the Vulcans' action with the InfVulcs' gun firing 20 of their bullets (objects/vulcan).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local vulcan = require("../../objects/vulcan/vulcan")

return define.chip {
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
