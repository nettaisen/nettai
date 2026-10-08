--!strict
-- Muramasa (EXE4 chip 0x0CA; its record at 0x080197EC + 0xCA * 0x2C): the swords' action 0x0A (lib/swords), its
-- variant 8: a slash on the two panels ahead (the region word 0x0705FF02: region 2, hitting navis, row 7 its own), hit
-- modifier 3, the effect table's row 0x4A (the slash 0c-14, its animation 1 in palette 8), the plain swing (0x8E). Its
-- damage is the HP its user has lost, at most 999 (its power 1046: formula 46, 0x080195AE, EXE5's formula 46).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local HIT = { hits = collision.hits_navis, is = collision.slash }
local SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 8 }

local muramasa: Chip = {
    damage = { formula = "hp_lost", cap = 999 },
    action = swords.slash {
        id = "muramasa/action",
        blade = swords.blade,
        hit = { region = regions.long, collision = HIT, hit_mod = 3 },
        effect = SLASH,
        sound = swords.swing,
    },
    codes = { "M" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 81,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "damage_shown_variable", "library", "variable_damage" },
}

return { muramasa = muramasa }
