--!strict
-- TimeBom2 (BN5 chip 0x04c): a bomb on the enemy's frontmost panel of a random row that counts 3, 2, 1 and blows
-- up the whole enemy area unless it is broken first (objects/timebom: BN6's TimeBom with BN5's placement).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local timebom = require("../../objects/timebom/timebom")

return define.chip {
    id = "bn5:timebom2",
    codes = { "D", "H", "Z" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 54,
    damage = 140,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = timebom.controller.hook(timebom.variants.bomb),
}
