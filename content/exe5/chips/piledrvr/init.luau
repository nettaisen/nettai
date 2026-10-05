--!strict
-- PileDrvr (EXE5 chip 0x15d): a Program Advance (GunDelS3, GunDelS3 and DjangoSP or DjangoDS) whose cut-in sets down a
-- pile ahead of the user and another at the far end of the row, turned around, each beaming ten times at the first of
-- the other side's bodies it sees down the row (./controller: EXE5's effect object #0x6F, EXE6's code that no EXE6 chip
-- uses).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")
local recipe_django_ds = require("./django").django_ds
local recipe_django_sp = require("./django").django_sp
local recipe_gundels3 = require("./gundels").gundels3

return define.chip {
    id = "piledrvr",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 30,
    hit_param = 130,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 32, sequence = { recipe_gundels3, recipe_gundels3, recipe_django_sp } },
        { order = 33, sequence = { recipe_gundels3, recipe_gundels3, recipe_django_ds } },
    },
    dimming = controller.hook,
}
