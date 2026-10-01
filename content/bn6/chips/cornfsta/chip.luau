--!strict
-- CornFsta: the Program Advance of three CornShts of a code (or of a
-- run): a cut-in in which the user's stand-in sows a field of bursting
-- corn among the enemies (docs/engine/dimming-chip-effects.md §16).

local controller = require("./controller")
local cornsht = require("../cornsht/chips")

return define.chip {
    id = "cornfsta",
    name = "CornFsta",
    description = "Explosive\ncorn with\n9pnl rch!",
    codes = { "A", "B", "C", "*" },
    element = "wood",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 10,
    lockout = 20,
    flags = { "dimming", "has_damage", "library" },
    library = { number = 332, index = 13, sort = 0 },
    slot_in_limit = 1,
    program_advances = {
        { order = 34, code_run = { chip = cornsht.cornsht1, count = 3 } },
        { order = 35, code_run = { chip = cornsht.cornsht2, count = 3 } },
        { order = 36, code_run = { chip = cornsht.cornsht3, count = 3 } },
    },
    dimming = controller.hook,
}
