--!strict
-- Mine (BN5 chip 0x062): a land mine hidden on a random panel of the other side's area: BN6's Mine
-- (@exelib/mine, the dimming spawner's entry 11; BN5's the same code but for its blast's effect number, BN5's
-- 0x2A for BN6's 0x47, the same look). Its collision type is BN6's (row 0x33 with the 0x80 self bit), which BN5's
-- targets don't test (docs/design/bn5-map.md §15.3 item 9).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("./controller")

return define.chip {
    id = "mine",
    codes = { "H", "P", "T" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 2,
    mb = 42,
    damage = 300,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = controller.hook,
}
