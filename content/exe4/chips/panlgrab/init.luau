--!strict
-- PanlGrab (EXE4 chip 0x074; its record at 0x080197EC + 0x74 * 0x2C): a dimming chip (action 0x0C's variant 0, its
-- parameter 0) that drops a grab shot on the first panel past the user's side's front in its row: lib/grab.

local grab = require("../lib/grab")

local chip: Chip = {
    codes = { "K", "O", "Y", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 10,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = grab.panel,
}

return { panlgrab = chip }
