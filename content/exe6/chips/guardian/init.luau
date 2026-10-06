--!strict
-- Guardian: a statue in front of the user. Broken by one side's attack, it
-- strikes back at that side's navi wherever it stands
-- (docs/engine/dimming-chip-effects.md §5).

local controller = require("@self/controller")

local chip: Chip = {
    codes = { "O" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 64,
    damage = 200,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook,
}

return { guardian = chip }
