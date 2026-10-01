--!strict
-- MetrKnuk: a giga cut-in chip that rains 16 fists on and around the
-- enemies, cracking the panels (docs/engine/dimming-chip-effects.md §18).
-- (The record's extra flag bytes 0x04 and 0x10 are read by no battle
-- routine.)

local controller = require("./controller")

return define.chip {
    id = "metrknuk",
    name = "MetrKnuk",
    codes = { "N" },
    element = "null", family = "break", class = "giga",
    rarity = 4, mb = 90, damage = 100, hit_param = 148, lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    library = { number = 307, index = 2, sort = 315 },
    slot_in_limit = 1,
    dimming = controller.hook,
}
