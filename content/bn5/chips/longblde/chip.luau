--!strict
-- LongBlde (BN5 chip 0x034): LongSwrd's slash, stronger: BN6's slash (@bn6/lib/swords/slash, the original's action
-- 0x13; BN5's the same code) with BN5's parts (lib/swords) and its slash effect 0x1A.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local regions = require("@bn6/lib/regions")
local slash = require("@bn6/lib/swords/slash")
local swords = require("../../lib/swords")

return define.chip {
    id = "bn5:longblde",
    codes = { "E", "L", "P" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 40,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = slash.action {
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.long, collision = swords.slashes_navis, hit_mod = 3 },
        effect = swords.effects.long_blade,
        sound = swords.swing,
    },
}
