--!strict
-- Muramasa (EXE5 chip 0x0ca): a slash on the two panels ahead, as strong as the HP its user lost: EXE6's slash
-- (@exelib/swords/slash, the original's action 0x13 with subtype 8) with EXE5's parts (lib/swords) and its slash
-- effect (0x2D, EXE5's palette 8 for EXE6's 6), its damage EXE5's formula 46 (0x0800EA78: EXE6's formula 20 but for its
-- cap, 999 for 500).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local regions = require("@exelib/regions")
local swords = require("../lib/swords")

return define.chip {
    id = "muramasa",
    codes = { "M" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 81,
    damage = { formula = "hp_lost", cap = 999 },
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "damage_shown_variable", "library", "variable_damage" },
    -- (The custom screen shows its damage as "???" only on an A, which it never comes as.)
    traits = { "hides_damage_as_a" },
    slot_in_limit = 1,
    megaman = "dark",
    action = swords.slash {
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.long, collision = swords.slashes_navis, hit_mod = 3 },
        effect = define.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 8 },
        sound = swords.swing,
    },
}
