--!strict
-- DrkRecov (EXE4 chip 0x0C2): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). The
-- recoveries' action 0x1D, its variant 8: the navi heals 1000 HP. Its cost: a worse custom drain and an HP drain
-- (rules/dark_chips).

local recoveries = require("../lib/recoveries")

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
    action = recoveries.action { id = "drkrecov/action", hp = 1000 },
}

return { drkrecov = drkrecov }
