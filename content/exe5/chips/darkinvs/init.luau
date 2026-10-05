--!strict
-- DarkInvs (EXE5 chip 0x0bd): a dark chip whose cut-in turns its user dark and gives it to the auto battle AI for
-- 600 ticks, untouchable (./controller: EXE5's effect object #0x18; its end, EXE5's action 0x49: ./drive-end).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

return define.chip {
    id = "darkinvs",
    codes = { "I", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "dark" },
    slot_in_limit = 1,
    dimming = controller.hook,
}
