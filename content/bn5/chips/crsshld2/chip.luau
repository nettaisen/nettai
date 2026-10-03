--!strict
-- CrsShld2 (BN5 chip 0x0aa): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- cross along the row that strikes the first navi it meets: BN5's guard (lib/guard, action 0x1F with subtype 6).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local guard = require("../../lib/guard")

return define.chip {
    id = "bn5:crsshld2",
    codes = { "C", "M", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 160,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = guard.looks.crsshld2, counter = { kind = "cross", palette = 1 } },
}
