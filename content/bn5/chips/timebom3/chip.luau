--!strict
-- TimeBom3 (BN5 chip 0x04d): a bomb on the enemy's frontmost panel of a random row that counts 3, 2, 1 and blows
-- up the whole enemy area unless it is broken first (objects/timebom: BN6's TimeBom with BN5's placement).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local timebom = require("../../objects/timebom/timebom")

return define.chip {
    id = "bn5:timebom3",
    codes = { "F", "H", "S" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 76,
    damage = 200,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = timebom.controller.hook(timebom.variants.bomb),
}
