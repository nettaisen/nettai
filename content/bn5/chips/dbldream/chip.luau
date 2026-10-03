--!strict
-- DblDream (BN5 chip 0x178): two big slashes over the column ahead and the next (NeoVari's pick: no library chip): the
-- shared slash (@common/swords/slash, BN5's action 0x13 subtype 11) with BN5's parts (lib/swords), region 0x11, hit
-- modifier 1, the big swing and LifeSrd's slash.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local regions = require("@common/regions")
local swords = require("../../lib/swords")

return define.chip {
    id = "bn5:dbldream",
    codes = { "E", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "special",
    rarity = 0,
    mb = 8,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = swords.slash {
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.wide_long, collision = swords.slashes_navis, hit_mod = 1 },
        effect = swords.effects.big,
        sound = swords.big_swing,
        swings = 2,
    },
}
