--!strict
-- CustSwrd (EXE4 chip 0x036; its record at 0x080197EC + 0x36 * 0x2C): the swords' action 0x0A (lib/swords), its
-- variant 7: a big slash over the two columns ahead (the region word 0x0705FF11: region 0x11, hitting navis, row 7
-- its own), hit modifier 3, the effect table's row 0x41 (the big slash 0c-15 in palette 1), the plain swing (0x8E).
-- Its damage is the custom gauge's (its power 1045: formula 45, 0x0801955A: EXE6's and EXE5's gauge formula, but a
-- full gauge gives 0; the side's own gauge only under event flag 0x1187, which no netbattle sets).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local HIT = { hits = collision.hits_navis, is = collision.slash }
local SLASH = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 1 }

local custswrd: Chip = {
    damage = { formula = "gauge", full = 0 },
    action = swords.slash {
        id = "custswrd/action",
        blade = swords.blade,
        hit = { region = regions.wide_long, collision = HIT, hit_mod = 3 },
        effect = SLASH,
        sound = swords.swing,
    },
    codes = { "B", "P", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 20,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "standard_library", "damage_shown_variable", "library", "variable_damage" },
}

return { custswrd = custswrd }
