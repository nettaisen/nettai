--!strict
-- PoisPhar: Anubis's statue (chips/anubis) draining every tick for 1441
-- ticks; the Program Advance of Anubis and two PoisSeeds
-- (docs/engine/dimming-chip-effects.md §7).

local controller = require("./anubis/controller")
local statue = require("./anubis/statue")
local anubis = require("./anubis").anubis
local poisseed = require("./poisseed").poisseed

-- Its statue (`byte_80CF0C0`'s row 1): a drain every tick, 100 HP, 1441
-- ticks.
local STATUE = statue.variant {
    anim = 1,
    period = 1,
    hp = 100,
    lifetime = 1441,
    identity = new.identity {
        id = "poisphar",
        class = "field_object",
        version = 3,
        ai_index = 1,
        look = { sprite = asset.sprite("anubis"), anim = 1, palette = 0, shadow = true },
    },
}

local chip: Chip = {
    codes = {},
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 1,
    hit_param = 128,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    program_advances = { { order = 2, sequence = { poisseed, poisseed, anubis } } },
    dimming = controller.hook(STATUE),
}

return { poisphar = chip }
