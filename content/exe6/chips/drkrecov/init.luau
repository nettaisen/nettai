--!strict
-- DrkRecov: the recovery chips' heal (content/exelib/recov/heal.luau) of 1000 HP.

local heal = require("@exelib/recov/heal")
local recov_chips = require("./recov")

local chip: Chip = {
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
    slot_in_limit = 1,
    dark_substitute = recov_chips.recov10,
    hp_bug = 7,
    action = heal.action { id = "drkrecov/action", hp = 1000 },
}

return { drkrecov = chip }
