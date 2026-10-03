--!strict
-- Invisibl (BN5 chip 0x085): its user turns invisible for six seconds: BN6's Invisibl (@bn6/chips/invisibl, the
-- dimming spawner's entry 40; BN5's the same code), 360 ticks (the record's parameter).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local invisible = require("@bn6/chips/invisibl/controller")

return define.chip {
    id = "bn5:invisibl",
    codes = { "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 2,
    mb = 12,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 2, 128 },
    slot_in_limit = 3,
    dimming = invisible.hook(360),
}
