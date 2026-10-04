--!strict
-- LongSwrd (BN5 chip 0x032): a slash over the two panels ahead: the shared slash (@exelib/swords/slash) with BN5's
-- parts (lib/swords).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local regions = require("@exelib/regions")
local swords = require("../lib/swords")

return define.chip {
    id = "longswrd",
    codes = { "C", "M", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = swords.slash {
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.long, collision = swords.slashes_navis, hit_mod = 3 },
        effect = swords.effects.long,
        sound = swords.swing,
    },
}
