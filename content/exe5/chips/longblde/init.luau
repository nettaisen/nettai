--!strict
-- LongBlde (EXE5 chip 0x034): LongSwrd's slash, stronger: the shared slash (@exelib/swords/slash, the original's action
-- 0x13; EXE5's the same code) with EXE5's parts (lib/swords) and its slash effect 0x1A.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local chip: Chip = {
    codes = { "E", "L", "P" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 40,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = swords.slash {
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.long, collision = swords.slashes_navis, hit_mod = 3 },
        effect = swords.effects.long_blade,
        sound = swords.swing,
    },
}

return { longblde = chip }
