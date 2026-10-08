--!strict
-- PopUp (EXE4 chip 0x086): its user goes under the ground for eight seconds, popping up for what it does: the dimming
-- chips' action 0x0C, its variant 0x1E (0x080220E0's), its controller ./controller. (Its power, 40, nothing reads.)
-- The opponent's Rush cancels it (lib/supports/rush). Its record is written by tools/exe4/gen_content.py
-- (bn6battle-verify) from the EXE4 ROMs.

local controller = require("@self/controller")

local chip: Chip = {
    codes = { "J", "P", "Y", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 2,
    mb = 21,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 2 },
    dimming = controller,
}

return { popup = chip }
