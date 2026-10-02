--!strict
-- Falzar: a giga cut-in chip that sends the cyber beast Falzar's feathers
-- diving onto the enemies' area, then summons Falzar, whose whirlwind
-- sweeps the three columns ahead of its user (./controller, ./feather,
-- ./summon, ./tornado; docs/engine/beast-chips.md). Its use is the
-- Japanese ROMs': the US ROMs have its record and a null routine
-- (`off_802CCB4` entry 35). (The record's extra flag byte 0x10 is read by
-- no battle routine of the US ROMs.)

local controller = require("./controller")

return define.chip {
    id = "falzar",
    name = "Falzar",
    description = "Falzar's\nruinous\ntornado!",
    codes = { "X" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 99,
    damage = 100,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    extra_flags = { 16 },
    library = { number = 313, index = 13, sort = 205 },
    slot_in_limit = 1,
    dimming = controller.hook,
}
