--!strict
-- DblHero: the Program Advance of WideBlde, LongBlde and ProtoMn[SP]: a
-- cut-in in which MegaMan and ProtoMan stand in for the user, MegaMan's
-- ten volleys hitting every enemy panel (docs/engine/dimming-chip-effects.md
-- §17).

local controller = require("@self/controller")
local longblde = require("./longblde")
local protoman_chips = require("./protoman")
local wideblde = require("./wideblde")

return define.chip {
    id = "dblhero",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 60,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    program_advances = { { order = 5, sequence = { wideblde, longblde, protoman_chips.protomn_sp } } },
    dimming = controller.hook,
}
