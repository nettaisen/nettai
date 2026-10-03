--!strict
-- InfVulc3 (BN5 chip 0x145): the Program Advance of three Vulcan3s with consecutive codes: action 0x19 with subtype
-- 6, the Vulcans' action with the InfVulcs' gun firing 28 of their bullets (objects/vulcan).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local vulcan = require("../../objects/vulcan/vulcan")

return define.chip {
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
