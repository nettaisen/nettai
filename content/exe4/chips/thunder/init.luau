--!strict
-- Thunder1, Thunder2 and Thunder3 (EXE4 chips 0x039 to 0x03B; their records at 0x080197EC + n * 0x2C): action 0x26
-- (0x080ECE14): a thunder ball (./ball) that homes in on the other side's navi for 5, 7 or 9 panels and paralyzes what
-- it hits for 90, 120 or 150 ticks (their parameters: 1, the panels, the status 0x10 to 0x12).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local statuses = require("../rules/status")
local thunder = require("@self/ball")

local thunder1: Chip = {
    action = thunder.shoot { id = "thunder1/action", ball = { panels = 5, status = statuses.paralyze_90 } },
    codes = { "B", "L", "P", "*" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 7,
    damage = 40,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local thunder2: Chip = {
    action = thunder.shoot { id = "thunder2/action", ball = { panels = 7, status = statuses.paralyze_120 } },
    codes = { "H", "S", "P" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 18,
    damage = 60,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local thunder3: Chip = {
    action = thunder.shoot { id = "thunder3/action", ball = { panels = 9, status = statuses.paralyze_150 } },
    codes = { "I", "T", "W" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 33,
    damage = 80,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { thunder1 = thunder1, thunder2 = thunder2, thunder3 = thunder3 }
