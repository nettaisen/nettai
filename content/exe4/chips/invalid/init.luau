--!strict
-- The invalid chip (EXE4 chip 0x184, past the named ones, no name): what a pick past the navi's Mega or Giga limit, or
-- in a code the chip doesn't come in, counts as (0x08020306: 0x3784, code 27); the custom screen refuses it
-- (rules/custom_screen's `invalid_picks`). Its use is the spawners' action 0x20, its variant 4: a plus chip's
-- (lib/plus), its parameter 0.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local plus = require("../lib/plus")
local spawners = require("../lib/spawners")

local invalid: Chip = {
    codes = {},
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = {},
    action = spawners.action { id = "invalid/action", spawn = plus.spawner(0) },
}

return { invalid = invalid }
