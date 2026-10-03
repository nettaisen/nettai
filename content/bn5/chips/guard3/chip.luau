--!strict
-- Guard3 (BN5 chip 0x05a): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- shock wave back along the row with the chip's damage: BN5's guard (lib/guard, action 0x1F with subtype 2).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local guard = require("../../lib/guard")

return define.chip {
    id = "bn5:guard3",
    codes = { "C", "M", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 16,
    damage = 150,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = guard.looks.guard3, counter = { kind = "wave" } },
}
