--!strict
-- WideSwrd (BN5 chip 0x031): a slash over the column ahead: BN6's slash (@bn6/lib/swords/slash) with BN5's parts
-- (lib/swords).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local regions = require("@bn6/lib/regions")
local slash = require("@bn6/lib/swords/slash")
local swords = require("../../lib/swords")

return define.chip {
    id = "wideswrd",
    codes = { "K", "S", "Z" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 1,
    mb = 16,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = slash.action {
        blade = swords.blade,
        hit = { region = regions.wide, collision = swords.slashes_navis, hit_mod = 3 },
        effect = swords.effects.wide,
        sound = swords.swing,
    },
}
