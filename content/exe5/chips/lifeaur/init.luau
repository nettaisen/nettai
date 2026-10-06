--!strict
-- LifeAur (EXE5 chip 0x0c9): an aura on the user that stops every hit of less than 200 for 3000 ticks: EXE6's barrier
-- chips (the dimming handler's subtype 14) with EXE5's barriers (lib/barriers).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local barriers = require("../lib/barriers")

local chip: Chip = {
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 70,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    dimming = barriers.controller.hook(barriers.life_aura),
    auto_battle = { class = "when_unbarriered" },
}

return { lifeaur = chip }
