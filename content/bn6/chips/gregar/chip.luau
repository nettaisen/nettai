--!strict
-- Gregar: a giga cut-in chip that summons the cyber beast Gregar, which
-- breathes flames over the three columns ahead of its user and drops rocks
-- on the enemies' area (./controller, ./summon, ./flame;
-- docs/engine/beast-chips.md). Its use is the Japanese ROMs': the US ROMs
-- have its record and a null routine (`off_802CCB4` entry 34). Its
-- parameter byte (100) is the rocks' damage; its own damage the flames'.
-- (The record's extra flag byte 0x10 is read by no battle routine of the
-- US ROMs.)

local controller = require("./controller")

return define.chip {
    id = "gregar",
    name = "Gregar",
    description = "Gregar's\nbreath\nattack!",
    codes = { "X" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 99,
    damage = 300,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    extra_flags = { 16 },
    library = { number = 312, index = 12, sort = 82 },
    slot_in_limit = 1,
    dimming = controller.hook { rock_damage = 100 },
}
