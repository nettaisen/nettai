--!strict
-- Anubis (EXE5 chip 0x0cc): a statue in front of the user that drains every enemy navi 1 HP each 6 ticks for 3000
-- ticks, with bubbles over the enemy's area: EXE6's Anubis with EXE5's statue (chips/anubis/anubis).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local anubis = require("@self/anubis")

-- Its statue (row 0 of 0x080D0858): a drain every 6 ticks, 100 HP.
local STATUE = anubis.variant { anim = 0, period = 6, hp = 100, version = 2 }

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
    dimming = anubis.controller.hook(STATUE),
    auto_battle = { class = "landing_ahead" },
}

return { anubis = chip }
