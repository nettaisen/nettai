--!strict
-- Anubis: a statue in front of the user that drains every enemy navi 1 HP
-- each 6 ticks for 3000 ticks, with bubbles over the enemy's area
-- (docs/engine/dimming-chip-effects.md §7).

local controller = require("@self/controller")
local statue = require("@self/statue")

-- Its statue (`byte_80CF0C0`'s row 0): a drain every 6 ticks, 100 HP, 3000
-- ticks.
local STATUE = statue.variant {
    anim = 0,
    period = 6,
    hp = 100,
    lifetime = 3000,
    identity = new.identity {
        id = "anubis",
        class = "field_object",
        version = 2,
        ai_index = 1,
        look = { sprite = asset.sprite("anubis"), anim = 0, palette = 0, shadow = true },
    },
}

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
    dimming = controller.hook(STATUE),
}

return { anubis = chip }
