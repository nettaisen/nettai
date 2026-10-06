--!strict
-- BigNoise (EXE5 chip 0x15a): the Program Advance of two Pulsar3s and ShadeMnSP or ShadeMnDS: ShadeMan cuts in and cries
-- out a noise over the three columns ahead of him that paralyzes (./bignoise, the summon table's entry 20).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bignoise = require("@self/bignoise")
local recipe_pulsar3 = require("./pulsar").pulsar3
local recipe_shademn_ds = require("./shademan")["shademn-ds"]
local recipe_shademn_sp = require("./shademan")["shademn-sp"]

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 34, sequence = { recipe_pulsar3, recipe_pulsar3, recipe_shademn_sp } },
        { order = 35, sequence = { recipe_pulsar3, recipe_pulsar3, recipe_shademn_ds } },
    },
    navi = bignoise.summon,
}

return { bignoise = chip }
