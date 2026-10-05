--!strict
-- CrosSwrd (EXE5 chip 0x176): a slash over the X ahead and one over the panel ahead (NeoVari's pick: no library
-- chip): the shared slash (@exelib/swords/slash, EXE5's action 0x13 subtype 10) with EXE5's parts (lib/swords), regions
-- 0x0B and 1 and the effect table's row 0x37.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local regions = require("@exelib/regions")
local swords = require("../lib/swords")

return define.chip {
    id = "crosswrd",
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
        effect = define.effect { sprite = asset.sprite("cross-slash"), anim = 0, palette = 1 },
        sound = swords.swing,
    },
}
