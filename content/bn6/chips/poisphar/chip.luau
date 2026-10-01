--!strict
-- PoisPhar: Anubis's statue (chips/anubis) draining every tick for 1441
-- ticks; the Program Advance of Anubis and two PoisSeeds
-- (docs/engine/dimming-chip-effects.md §7).

local controller = require("../anubis/controller")
local statue = require("../anubis/statue")
local anubis = require("../anubis/chip")
local poisseed = require("../poisseed/chip")

return define.chip {
    id = "poisphar",
    name = "PoisPhar",
    description = "Places a\npoisoned\nPharaoh",
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
    library = { number = 342, index = 23, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 2, sequence = { poisseed, poisseed, anubis } } },
    dimming = controller.hook(statue.poisphar),
}
