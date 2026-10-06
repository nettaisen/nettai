--!strict
-- Meddy's own chip, MeddyCap (EXE5 chip 0x19A), which her custom screen offers until it is picked (the navi's
-- `own_chip`): her own action 7, her throw (./throw), with its record's parameters: a capsule of variant 5 (it
-- paralyzes for 150 ticks; its hit modifier 0), bursting along its column, not running while dimmed and not halved
-- (its third parameter is 1). Its damage is by her level (the chips' damage formula 60: the damage rows' row 11).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi_level = require("../../lib/navi_level")
local throw = require("./throw")

return define.chip {
    id = "meddycap",
    codes = { "M" },
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 80, 80, 80, 80, 80, 80, 100 }),
    hit_param = 148,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = throw.action("meddycap/action", { variant = 5, line = 0, mode = 1 }),
}
