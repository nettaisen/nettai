--!strict
-- LifeSrd (EXE4 chip 0x158, a Program Advance's; VarSwrd's pick too): the swords' action 0x0A (lib/swords), its
-- variant 5: a big slash over the two columns ahead (the region word 0x0705FF11), hit modifier 3, the effect table's
-- row 0x1D (the big slash 0c-15), the big swing (0xB4). Its recipes are the recipe table's first two (the pointer list at
-- 0x080212B0, OK's 0x0801F390): Sword, WideSwrd and LongSwrd, or Sword, WideBlde and LongBlde.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")
local blades = require("./sword")

local HIT = { hits = collision.hits_navis, is = collision.slash }
local BIG_SLASH = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 0 }

local lifesrd: Chip = {
    action = swords.slash {
        id = "lifesrd/action",
        blade = swords.blade,
        hit = { region = regions.wide_long, collision = HIT, hit_mod = 3 },
        effect = BIG_SLASH,
        sound = swords.big_swing,
    },
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 0, sequence = { blades.sword, blades.wideswrd, blades.longswrd } },
        { order = 1, sequence = { blades.sword, blades.wideblde, blades.longblde } },
    },
}

return { lifesrd = lifesrd }
