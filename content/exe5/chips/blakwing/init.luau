--!strict
-- BlakWing (EXE5 chip 0x0d6): a cut-in chip: perches over the enemy's columns, and 18 wings falling down them, one
-- every 12 ticks (./blakwing, the dimming handler's subtype 32).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local blakwing = require("@self/blakwing")

return define.chip {
    id = "blakwing",
    codes = { "W" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 58,
    damage = 20,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    megaman = "dark",
    dimming = blakwing.hook,
}
