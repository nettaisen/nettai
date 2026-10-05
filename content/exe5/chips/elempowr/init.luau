--!strict
-- ElemPowr (EXE5 chip 0x121): a cut-in chip that takes the power of the panel type the user stands on: 10 Atk+ for each
-- panel of it on the field (at most 90), and they all turn back to normal (./controller, the dimming handler's
-- subtype 56).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

return define.chip {
    id = "elempowr",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 64,
    damage = 0,
    hit_param = 50,
    lockout = 0,
    flags = { "dimming" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = controller.hook,
}
