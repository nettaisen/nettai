--!strict
-- LongSwrd (EXE5 chip 0x032): a slash over the two panels ahead: the shared slash (@exelib/swords/slash) with EXE5's
-- parts (lib/swords).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local regions = require("@exelib/regions")
local swords = require("../lib/swords")

-- The slash over two panels (effect 0x17).
local LONG_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 0 }

local chip: Chip = {
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
        effect = LONG_SLASH,
        sound = swords.swing,
    },
    auto_battle = { class = "into_reach", range = "two_in_row" },
}

return { longswrd = chip }
