--!strict
-- DrkRecov (EXE4 chip 0x0C2): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). The
-- recoveries' action 0x1D (chips/recov/recov), its variant 8: the navi heals 1000 HP (its counter byte 0: no mood). Its cost: a
-- worse custom drain and an HP drain (rules/dark_chips).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local recov = require("./recov/recov")

local drkrecov: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dark" },
    action = recov.action { id = "drkrecov/action", hp = 1000, mood = 0 },
}

return { drkrecov = drkrecov }
