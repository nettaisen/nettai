--!strict
-- Anubis (EXE5 chip 0x0cc): a statue in front of the user that drains every enemy navi 1 HP each 6 ticks for 3000
-- ticks, with bubbles over the enemy's area: EXE6's Anubis with EXE5's statue (chips/anubis/anubis).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local anubis = require("@self/anubis")

local chip: Chip = {
    codes = { "A" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 4,
    mb = 86,
    damage = 1,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    megaman = "dark",
    dimming = anubis.controller.hook(anubis.variants.anubis),
}

return { anubis = chip }
