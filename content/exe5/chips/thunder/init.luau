--!strict
-- Thunder (EXE5 chip 0x013): a thunder ball that homes in on the enemy for 5 panels and paralyzes what it hits:
-- EXE6's Thunder (@exelib/thunder) firing EXE5's ball (chips/thunder/ball).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local statuses = require("../rules/status")
local thunder = require("@self/ball")

local chip: Chip = {
    codes = { "B", "L", "P", "*" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 7,
    damage = 40,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = thunder.shoot { ball = { panels = 5, status = statuses.paralyze_90 } },
}

return { thunder = chip }
