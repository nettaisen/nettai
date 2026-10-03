--!strict
-- GigaCan2 (BN5 chip 0x141): the Program Advance of three HiCannons of one code (or Attck+10 and two of them): BN6's
-- cannon (@common/cannon, action 0x14 with subtype 5: the arm's row 1 of 0x080EC2AC, the shot's row 0x0C of 0x080EC2B3)
-- firing BN5's GigaCan shot (objects/projectile: a blast, no burst), the cannon and the navi leaving
-- afterimages as it charges. (BN5 adds a third afterimage, of sprite 0c-58, when the side's NaviStats +0x4C, its
-- spread program, is set: lib/arm's reading, 0, as the engine models no such program.)
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local shooter = require("../../lib/cannon")
local variants = require("../../objects/projectile/variants")
local recipe_attck_10 = require("../attck-10/chip")
local recipe_hicannon = require("../hicannon/chip")

return define.chip {
    id = "bn5:gigacan2",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 500,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 1, sequence = { recipe_attck_10, recipe_hicannon, recipe_hicannon }, per_player_gauges_only = true },
        { order = 37, code_run = { chip = recipe_hicannon, count = 3 } },
    },
    action = shooter.action {
        id = "bn5:gigacan2/action",
        arm = shooter.arms.hicannon,
        shot = variants.gigacan,
        afterimages = true,
    },
}
