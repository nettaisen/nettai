--!strict
-- Atk+30 (EXE4 chip 0x0CD; its record at 0x080197EC + 0xCD * 0x2C): folded into the chip before it on the custom
-- screen, 30 more damage; used alone, the spawners' action 0x20, its variant 4, a plus chip's (lib/plus), its
-- parameter 0: in a link battle the side's Atk+ bonus, else the sparkle.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local plus = require("../lib/plus")
local spawners = require("../lib/spawners")

local chip: Chip = {
    modifier = "attack_plus",
    action = spawners.action { id = "atk-30/action", spawn = plus.spawner(0) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "mega",
    rarity = 3,
    mb = 66,
    damage = 30,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
}

return { ["atk-30"] = chip }
