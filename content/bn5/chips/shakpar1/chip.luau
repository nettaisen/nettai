--!strict
-- ShakPar1 (BN5 chip 0x14c): the Program Advance of two Voltz1s and a SpShake1 (or three SpShake1s of one code): three
-- SpShake waves along the rows from the back column, 15 ticks apart (objects/shakpar, instant effect 22; the record's
-- parameters: speed row 2, hits that paralyze, palette 0).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local shakpar = require("../../objects/shakpar/shakpar")
local spshake = require("../../objects/spshake/spshake")
local recipe_spshake1 = require("../spshake1/chip")

return define.chip {
    id = "bn5:shakpar1",
    codes = { "A", "B", "C", "*" },
    element = "elec",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 80,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 48, code_run = { chip = recipe_spshake1, count = 3 } },
    },
    instant = shakpar.instant(spshake.wave(2, true, 0)),
}
