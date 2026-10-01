--!strict
-- DrkRecov: the recovery chips' heal (chips/recov/heal.luau) of 1000 HP.

local heal = require("../recov/heal")
local recov = require("../recov/chips")

return define.chip {
    id = "drkrecov",
    name = "DrkRecov",
    description = "\n\n",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 80,
    damage = 0,
    hit_param = 49,
    lockout = 0,
    extra_flags = { 16, 32 },
    library = { number = 0, index = 68, sort = 0 },
    slot_in_limit = 1,
    dark_substitute = recov[1],
    hp_bug = 7,
    action = heal.action { id = "drkrecov/action", hp = 1000 },
}
