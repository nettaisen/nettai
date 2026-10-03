--!strict
-- CustSwrd (BN5 chip 0x036): a big slash over the two columns ahead (region 0x11, as BN6's LifeSrd) whose damage is
-- the custom gauge's (BN5's formula 45, 0x0800EA20: BN6's `sub_8010B78`, the same code): the shared slash
-- (@common/swords/slash, BN6's action 0x13; BN5's the same code) with BN5's parts (lib/swords), its slash
-- effect 0x28 and the plain swing.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local regions = require("@common/regions")
local swords = require("../../lib/swords")

return define.chip {
    id = "bn5:custswrd",
    codes = { "E", "L", "R" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 20,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 3,
    damage = { formula = "gauge" },
    action = swords.slash {
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.wide_long, collision = swords.slashes_navis, hit_mod = 3 },
        effect = swords.effects.custom,
        sound = swords.swing,
    },
}
