--!strict
-- DrkLance (EXE4 chip 0x0BF): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). The
-- spawners' action 0x20, its variant 6: Lance's three lances (chips/lance/lance), its parameters' first byte 1, their
-- second palette. Its damage is half the opponent's maximum HP, at most 999 (its power 1050: damage formula 50),
-- worked out again every tick it waits in the hand (flag 0x80). Its cost: a worse custom drain (rules/dark_chips).

local lance = require("./lance/lance")
local spawners = require("../lib/spawners")

local drklance: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = { formula = "half_opponent_max_hp" },
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "dark", "variable_damage" },
    action = spawners.action {
        id = "drklance/action",
        spawn = function(user: Object, element: number, damage: number)
            lance.thrust(user, element, 1, damage)
        end,
    },
}

return { drklance = drklance }
