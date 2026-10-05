--!strict
-- DblBeast: a mega cut-in chip that sends Gregar's and Falzar's beasts
-- after the enemies while the user warps out
-- (docs/engine/dimming-chip-effects.md §19). (The record's extra flag
-- byte 0x10 is read by no battle routine.)

local controller = require("@self/controller")

return define.chip {
    id = "dblbeast",
    codes = { "W" },
    element = "null", family = "null", class = "mega",
    rarity = 4, mb = 99, damage = 0, hit_param = 138, lockout = 0,
    flags = { "dimming" },
    library = { number = 281, index = 11, sort = 137 },
    slot_in_limit = 1,
    dimming = controller.hook,
}
