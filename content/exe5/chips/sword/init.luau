--!strict
-- Sword (EXE5 chip 0x030): a slash on the panel ahead: the shared slash (@exelib/swords/slash) with EXE5's parts
-- (lib/swords).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local regions = require("@exelib/regions")
local swords = require("../lib/swords")

-- Its slash over the panel ahead (effect 0x18).
local SWORD_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 2, palette = 0 }

local chip: Chip = {
    codes = { "C", "L", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 9,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = swords.slash {
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.single, collision = swords.slashes_navis, hit_mod = 3 },
        effect = SWORD_SLASH,
        sound = swords.swing,
    },
    auto_battle = { class = "into_reach", range = "one_ahead" },
}

return { sword = chip }
