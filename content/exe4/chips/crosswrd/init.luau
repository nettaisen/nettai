--!strict
-- CrosSwrd (EXE4 chip 0x176, past the library: NeoVari's pick): the swords' action 0x0A (lib/swords), its variant 10:
-- a slash over the X ahead (the region word 0x0705FF0B) and one over the panel ahead (0x0705FF01, 0x080EB72C), both of
-- hit modifier 3, the effect table's row 0x62 (the cross slash 10-35 in palette 1), the plain swing (0x8E).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local HIT = { hits = collision.hits_navis, is = collision.slash }
local CROSS_SLASH = new.effect { sprite = asset.sprite("cross-slash"), anim = 0, palette = 1 }

local crosswrd: Chip = {
    action = swords.slash {
        id = "crosswrd/action",
        blade = swords.blade,
        hit = { region = regions.x, collision = HIT, hit_mod = 3 },
        second_hit = { region = regions.single, collision = HIT, hit_mod = 3 },
        effect = CROSS_SLASH,
        sound = swords.swing,
    },
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
}

return { crosswrd = crosswrd }
