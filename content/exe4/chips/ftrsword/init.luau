--!strict
-- FtrSword (EXE4 chip 0x172, past the library: VarSwrd's pick): the swords' action 0x0A (lib/swords), its variant 9: a
-- slash over the three panels ahead (the region word 0x0705FF06), hit modifier 3, the effect table's row 0x4B (the
-- slash 0c-14's animation 3), the plain swing (0x8E).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local HIT = { hits = collision.hits_navis, is = collision.slash }
local LINE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 3, palette = 0 }

local ftrsword: Chip = {
    action = swords.slash {
        id = "ftrsword/action",
        blade = swords.blade,
        hit = { region = regions.line, collision = HIT, hit_mod = 3 },
        effect = LINE_SLASH,
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

return { ftrsword = ftrsword }
