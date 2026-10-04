--!strict
-- HolyDrem (BN5 chip 0x133): a light MegaMan's Giga chip: in the dimming, a stand-in of the user fires a shot along
-- the row, then one more for each holy panel on the field, each panel turned Normal: 50 damage a shot
-- (docs/design/bn5-map.md §6.2). Its dimming controller (./controller, BN6's BurnSqr controller's code), stand-in
-- (./actor) and shot (./shot); the light MegaMan it needs is the record's `megaman` field, which the rules' light
-- and dark system reads.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("@self/controller")

return define.chip {
    id = "holydrem",
    codes = { "H" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 92,
    damage = 50,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    extra_flags = { 8, 16 },
    slot_in_limit = 1,
    megaman = "light",
    dimming = controller.hook,
}
