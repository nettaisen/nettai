--!strict
-- Anubis (EXE4 chip 0x0CC): the dimming chips' variant 0x18 (./anubis): a statue in front of the user that drains
-- every enemy navi 1 HP each 6 ticks for 3000 ticks, with bubbles over the enemy's area.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local anubis = require("@self/anubis")

-- Anubis's statue (0x080D82E8's row 0: animation 0, a drain every 6 ticks, 100 HP).
local VARIANT = anubis.variant { anim = 0, period = 6, hp = 100, lifetime = 3000 }

local chip: Chip = {
    codes = { "A" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 4,
    mb = 86,
    damage = 1,
    hit_param = 1,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = anubis.controller.hook(VARIANT),
}

return { anubis = chip }
