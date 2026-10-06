--!strict
-- Anubis: a statue in front of the user that drains every enemy navi 1 HP
-- each 6 ticks for 3000 ticks, with bubbles over the enemy's area
-- (docs/engine/dimming-chip-effects.md §7).

local controller = require("@self/controller")
local statue = require("@self/statue")

local chip: Chip = {
    codes = { "P" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 86,
    damage = 1,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook(statue.anubis),
}

return { anubis = chip }
