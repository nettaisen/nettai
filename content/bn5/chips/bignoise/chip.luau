--!strict
-- BigNoise (BN5 chip 0x15a): the Program Advance of two Pulsar3s and ShadeMnSP or ShadeMnDS: ShadeMan cuts in and cries
-- out a noise over the three columns ahead of him that paralyzes (./bignoise, the summon table's entry 20).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bignoise = require("./bignoise")
local recipe_pulsar3 = require("../pulsar/chips").pulsar3
local recipe_shademn_ds = require("../shademan/chips").shademn_ds
local recipe_shademn_sp = require("../shademan/chips").shademn_sp

return define.chip {
    id = "bn5:bignoise",
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
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 34, sequence = { recipe_pulsar3, recipe_pulsar3, recipe_shademn_sp } },
        { order = 35, sequence = { recipe_pulsar3, recipe_pulsar3, recipe_shademn_ds } },
    },
    navi = bignoise.summon,
}
