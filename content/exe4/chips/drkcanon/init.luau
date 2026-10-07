--!strict
-- DrkCanon (EXE4 chip 0x0BB): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). The
-- cannons' action 0x0B (lib/cannon), its variant 3: the dark cannon (attachment row 0x30), the cannon's shot (projectile row 1).
-- Its damage is the HP its user has lost, at most 999 (its power 1046: damage formula 46, 0x080195AE), worked out
-- again every tick it waits in the hand (flag 0x80). Its cost: the charged shot becomes weapon routine 0x21
-- (rules/dark_chips).

local cannon = require("../lib/cannon")
local variants = require("../objects/projectile/variants")

local drkcanon: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = { formula = "hp_lost", cap = 999 },
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "dark", "variable_damage" },
    action = cannon.action { id = "drkcanon/action", arm = cannon.arms.drkcanon, shot = variants.cannon },
}

return { drkcanon = drkcanon }
