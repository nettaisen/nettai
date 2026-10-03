--!strict
-- LifeSrd (BN5 chip 0x153): a big slash over the two columns ahead, the Program Advance of Sword, WideSwrd and
-- LongSwrd (or Sword, WideBlde and LongBlde) in that order: the shared slash (@common/swords/slash, action 0x13 with
-- subtype 5) with BN5's parts (lib/swords): its subtype's rows of BN5's tables (0x080EBF8C, 0x080EBFC0: the two
-- columns, a slash hitting navis, hit modifier 3; effect 0x1B by 0x080EC028) are BN6's.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local regions = require("@common/regions")
local swords = require("../../lib/swords")

return define.chip {
    id = "bn5:lifesrd",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = swords.slash {
        id = "bn5:lifesrd/action",
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.wide_long, collision = swords.slashes_navis, hit_mod = 3 },
        effect = swords.effects.big,
        sound = swords.big_swing,
    },
}
