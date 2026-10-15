--!strict
-- OmegaRkt (EXE5 chip 0x131): a giga cut-in chip that launches a rocket along the middle row, blowing up what it hits or the enemy's back two columns (./controller).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

local chip: Chip = {
    -- The version whose saves hold it (rules/folder; rules/builds' groups).
    version = "colonel",
    codes = { "R" },
    element = "null",
    family = "null",
    class = "giga",
    -- A save can collect it (rules/folder): Team Colonel's library lists it, where this record's flags are Team
    -- ProtoMan's ROM's, which doesn't.
    collectible = true,
    rarity = 4,
    mb = 89,
    damage = 270,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    extra_flags = { 4, 16 },
    slot_in_limit = 1,
    dimming = controller.hook,
    auto_battle = { class = "at_once" },
}

return { omegarkt = chip }
