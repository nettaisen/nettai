--!strict
-- Z Saver (EXE5 chip 0x0da): three slashes with growing reach, a fourth (a piercing sonic boom) for B and back in the
-- twelve ticks after the third's hit, A held: EXE5's action 0x34 (./action, EXE6's Z Saver with the Katanas' step).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local zsaver = require("@self/action")

return define.chip {
    id = "z-saver",
    codes = { "Z" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 80,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = zsaver.action,
}
