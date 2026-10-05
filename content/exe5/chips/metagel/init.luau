--!strict
-- MetaGel (EXE5 chip 0x05f): a cut-in chip that drops a gel from high above on each row's panel just ahead of the
-- user's area, one every 13 ticks in a shuffled order: each hits that panel and, where it stays, takes it for the
-- user's side: EXE5's controller (controller.luau, the dimming handler's subtype 6, effect object #0x21) and its gel
-- (gel.luau, attack object #0x45).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

return define.chip {
    id = "metagel",
    codes = { "F", "N", "T" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 52,
    damage = 120,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = controller.hook,
}
