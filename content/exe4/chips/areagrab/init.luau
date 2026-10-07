--!strict
-- AreaGrab (EXE4 chip 0x075; its record at 0x080197EC + 0x75 * 0x2C): a dimming chip (action 0x0C's variant 0, its
-- parameter 1) that drops grab shots on the other side's front column: lib/grab.

local grab = require("../lib/grab")

local chip: Chip = {
    codes = { "E", "M", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 10,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = grab.area,
}

return { areagrab = chip }
