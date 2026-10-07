--!strict
-- Navi+20 (EXE4 chip 0x096; its record at 0x080197EC + 0x96 * 0x2C): folded into the navi chip before it on the custom
-- screen, 20 more damage; used alone, the spawners' action 0x20, its variant 4, a plus chip's (lib/plus), its
-- parameter 1: in a link battle the side's Navi+ bonus, else the sparkle.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local plus = require("../lib/plus")
local spawners = require("../lib/spawners")

local chip: Chip = {
    modifier = "navi_plus",
    action = spawners.action { id = "navi-20/action", spawn = plus.spawner(1) },
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 2,
    mb = 36,
    damage = 20,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

return { ["navi-20"] = chip }
