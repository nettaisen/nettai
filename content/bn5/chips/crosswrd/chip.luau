--!strict
-- CrosSwrd (BN5 chip 0x176): a slash over the X ahead and one over the panel ahead (NeoVari's pick: no library chip): the
-- shared slash (@common/swords/slash, BN5's action 0x13 subtype 10) with BN5's parts (lib/swords), regions 0x0B and 1
-- and the effect table's row 0x37.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local regions = require("@common/regions")
local swords = require("../../lib/swords")

return define.chip {
    id = "bn5:crosswrd",
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
        hit = { region = regions.x, collision = swords.slashes_navis, hit_mod = 3 },
        second_hit = { region = regions.single, collision = swords.slashes_navis, hit_mod = 3 },
        effect = define.effect { sprite = asset.sprite("bn5:cross-slash"), anim = 0, palette = 1 },
        sound = swords.swing,
    },
}
