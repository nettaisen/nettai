--!strict
-- Guard2 (BN5 chip 0x059): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- shock wave back along the row with the chip's damage: BN5's guard (lib/guard, action 0x1F with subtype 1).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local guard = require("../../lib/guard")

return define.chip {
    id = "bn5:guard2",
    codes = { "B", "L", "Q", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 10,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = guard.looks.guard2, counter = { kind = "wave" } },
}
