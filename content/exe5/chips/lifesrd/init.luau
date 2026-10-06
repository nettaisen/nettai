--!strict
-- LifeSrd (EXE5 chip 0x153): a big slash over the two columns ahead, the Program Advance of Sword, WideSwrd and
-- LongSwrd (or Sword, WideBlde and LongBlde) in that order: the shared slash (@exelib/swords/slash, action 0x13 with
-- subtype 5) with EXE5's parts (lib/swords): its subtype's rows of EXE5's tables (0x080EBF8C, 0x080EBFC0: the two
-- columns, a slash hitting navis, hit modifier 3; effect 0x1B by 0x080EC028) are EXE6's.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local regions = require("@exelib/regions")
local swords = require("../lib/swords")
local recipe_longblde = require("./longblde").longblde
local recipe_longswrd = require("./longswrd").longswrd
local recipe_sword = require("./sword").sword
local recipe_wideblde = require("./wideblde").wideblde
local recipe_wideswrd = require("./wideswrd").wideswrd

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 21, sequence = { recipe_sword, recipe_wideswrd, recipe_longswrd } },
        { order = 22, sequence = { recipe_sword, recipe_wideblde, recipe_longblde } },
    },
    action = swords.slash {
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.wide_long, collision = swords.slashes_navis, hit_mod = 3 },
        effect = swords.effects.big,
        sound = swords.big_swing,
    },
}

return { lifesrd = chip }
