--!strict
-- TimeBom+ (BN5 chip 0x15b): the Program Advance of three TimeBoms (its recipe comes with BN5's Program
-- Advances): TimeBom1-3's bomb with 300 HP, its own look and a harder shake (objects/timebom: BN6's TimeBom with
-- BN5's placement).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local timebom = require("../../objects/timebom/timebom")

return define.chip {
    id = "bn5:timebom",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 600,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    dimming = timebom.controller.hook(timebom.variants.big),
}
