--!strict
-- RainyDay (BN5 chip 0x07e): a cut-in chip: a rain cloud over the first enemy navi ahead hits it once, and once more for
-- each sea panel of the user's side, which turns normal (./rainyday, the dimming handler's subtype 61).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local rainyday = require("@self/rainyday")

return define.chip {
    id = "rainyday",
    codes = { "B", "G", "L" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 47,
    damage = 40,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 3,
    dimming = rainyday.hook,
}
