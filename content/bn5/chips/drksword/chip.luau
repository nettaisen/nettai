--!strict
-- DrkSword (BN5 chip 0x0bc): a dark chip: a dark big slash over the two columns ahead, BN6's slash
-- (@bn6/lib/swords/slash) with BN5's parts (lib/swords). Its subtype's row (6) of BN5's tables (0x080EBF8C, 0x080EBFC0)
-- differs from BN6's: the hit is BN5's row 0x11 (BN6's 7) and carries bug 0x14 with argument 2. Its use costs buster
-- blanks (rules/light-dark).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local collision = require("../../rules/collision")
local regions = require("@bn6/lib/regions")
local slash = require("@bn6/lib/swords/slash")
local swords = require("../../lib/swords")

return define.chip {
    id = "bn5:drksword",
    codes = { "Z" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 400,
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "dark", "variable_damage" },
    slot_in_limit = 1,
    action = slash.action {
        id = "bn5:drksword/action",
        blade = swords.blade,
        hit = {
            region = regions.wide_long,
            collision = { is = collision.collision_11, hits = collision.hits_navis },
            hit_mod = 3,
            bug = 0x14,
            bug_arg = 2,
        },
        effect = swords.effects.dark,
        sound = swords.big_swing,
    },
}
