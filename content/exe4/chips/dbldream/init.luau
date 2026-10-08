--!strict
-- DblDream (EXE4 chip 0x178, past the library: NeoVari's pick): the swords' action 0x0A (lib/swords), its variant 11:
-- two big slashes over the two columns ahead (the region word 0x0705FF11; two swings, 0x080EB5E4), hit modifier 1, the
-- effect table's row 0x1D (the big slash 0c-15), the big swing (0xB4).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local HIT = { hits = collision.hits_navis, is = collision.slash }
local BIG_SLASH = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 0 }

local dbldream: Chip = {
    action = swords.slash {
        id = "dbldream/action",
        blade = swords.blade,
        hit = { region = regions.wide_long, collision = HIT, hit_mod = 1 },
        effect = BIG_SLASH,
        sound = swords.big_swing,
        swings = 2,
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

return { dbldream = dbldream }
