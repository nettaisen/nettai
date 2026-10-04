--!strict
-- Thunder (BN5 chip 0x013): a thunder ball that homes in on the enemy for 5 panels and paralyzes what it hits:
-- BN6's Thunder (@exelib/thunder) firing BN5's ball (chips/thunder/ball).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local statuses = require("../../rules/status")
local thunder = require("./ball")

return define.chip {
    id = "bn5:thunder",
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
