--!strict
-- DarkCirc (EXE5 chip 0x0bb): a dark chip: a gun sight circles the enemy's area until A or 360 ticks, then leaves
-- seven shots on the panels it passes, each with the HP bug 0x54 (argument 0x32): EXE5's CircGun (chips/circgun/circgun:
-- the shared CircGun, the dimming handler's subtype 42, with EXE5's look, CircGun's kinds) of its variant: period
-- 2, 7 shots, look 1. Its use costs custom damage (rules/light-dark).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local circgun = require("./circgun/circgun")

return define.chip {
    id = "darkcirc",
    codes = { "R" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 300,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "dark", "variable_damage" },
    slot_in_limit = 1,
    dimming = circgun.hook(circgun.variant { period = 2, shots = 7, look = 1 }),
}
