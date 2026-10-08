--!strict
-- Rush, Beat and Tango (EXE4 chips 0x17F to 0x181, past the folders' chips): the chips the NaviCust supports' telops name
-- (the roles `chips.rush`, `beat` and `tango`; their triggers, 0x0800C8C0, 0x0800C838 and 0x0800C7D8, pass them to the
-- supports' controller, lib/supports). Each is a dimming chip of its own, the dimming handler's variant 0x42 with its
-- support as its parameter (0, 1, 2): its use spawns the controller (`supports.dimming`), no one cutting in on it (the
-- handler's 0x080EBA26: a chip from 0x170 on). No folder holds them. Their records are written by
-- tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local supports = require("../lib/supports/controller")

local rush: Chip = {
    codes = { "*" },
    element = "null",
    family = "summon",
    class = "special",
    rarity = 0,
    mb = 6,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    traits = { "no_cut_in" },
    dimming = supports.dimming("rush"),
}

local beat: Chip = {
    codes = { "*" },
    element = "null",
    family = "summon",
    class = "special",
    rarity = 0,
    mb = 6,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    traits = { "no_cut_in" },
    dimming = supports.dimming("beat"),
}

local tango: Chip = {
    codes = { "*" },
    element = "null",
    family = "summon",
    class = "special",
    rarity = 0,
    mb = 6,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    traits = { "no_cut_in" },
    dimming = supports.dimming("tango"),
}

return { rush = rush, beat = beat, tango = tango }
