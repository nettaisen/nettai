--!strict
-- BoyBomb2 (BN5 chip 0x056): a bomb set on the panel ahead that blows up over the 3x3 around it after 60 ticks
-- (chips/boybomb/bomb, the dimming handler's subtype 58).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bomb = require("../boybomb/bomb")

return define.chip {
    id = "bn5:boybomb2",
    codes = { "H", "P", "W" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 180,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = bomb.hook,
}
