--!strict
-- Guardian (EXE5 chip 0x0cb): a statue in front of the user; broken by one side's attack, it strikes back at that
-- side's navi wherever it stands: EXE6's Guardian with EXE5's statue (chips/guardian/guardian: 10 HP).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local guardian = require("@self/guardian")

local chip: Chip = {
    codes = { "O" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 3,
    mb = 64,
    damage = 200,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    dimming = guardian.controller.hook,
}

return { guardian = chip }
