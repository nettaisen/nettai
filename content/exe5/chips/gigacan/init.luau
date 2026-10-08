--!strict
-- GigaCan1, GigaCan2 and GigaCan3: the gigacan series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local recipe_attck_10 = require("./attck-10")["attck-10"]
local recipe_cannon = require("./cannon").cannon
local recipe_hicannon = require("./cannon").hicannon
local recipe_m_cannon = require("./cannon")["m-cannon"]
local attachment = require("../objects/attachment")
local shooter = require("../lib/cannon")
local variants = require("../objects/projectile/variants")

-- Their cannons (the arm's rows 0 to 2 of 0x080EC2AC: the attachment table's, 0c-01 in palettes 0, 1 and 2, the
-- Cannons'), at the navi's attach point 7.
local GIGACAN1_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 0, attach_point = 7 }
local GIGACAN2_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 1, attach_point = 7 }
local GIGACAN3_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 2, attach_point = 7 }

-- GigaCan1 (EXE5 chip 0x140): the Program Advance of three Cannons of one code (or Attck+10 and two of them): EXE6's
-- cannon (@exelib/cannon, action 0x14 with subtype 4: the arm's row 0 of 0x080EC2AC, the shot's row 0x0C of 0x080EC2B3)
-- firing EXE5's GigaCan shot (objects/projectile: a blast, no burst), the cannon and the navi leaving
-- afterimages as it charges (a third, Hub Style's shade, in Hub Style: lib/cannon).
local gigacan1: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 0, sequence = { recipe_attck_10, recipe_cannon, recipe_cannon }, operation_battle_only = true },
        { order = 36, code_run = { chip = recipe_cannon, count = 3 } },
    },
    action = shooter.action {
        arm = GIGACAN1_ARM,
        shot = variants.gigacan,
        afterimages = true,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- GigaCan2 (EXE5 chip 0x141): the Program Advance of three HiCannons of one code (or Attck+10 and two of them): EXE6's
-- cannon (@exelib/cannon, action 0x14 with subtype 5: the arm's row 1 of 0x080EC2AC, the shot's row 0x0C of 0x080EC2B3)
-- firing EXE5's GigaCan shot (objects/projectile: a blast, no burst), the cannon and the navi leaving
-- afterimages as it charges (a third, Hub Style's shade, in Hub Style: lib/cannon).
local gigacan2: Chip = {
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
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 1, sequence = { recipe_attck_10, recipe_hicannon, recipe_hicannon }, operation_battle_only = true },
        { order = 37, code_run = { chip = recipe_hicannon, count = 3 } },
    },
    action = shooter.action {
        arm = GIGACAN2_ARM,
        shot = variants.gigacan,
        afterimages = true,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

-- GigaCan3 (EXE5 chip 0x142): the Program Advance of three M-Cannons of one code (or Attck+10 and two of them): EXE6's
-- cannon (@exelib/cannon, action 0x14 with subtype 6: the arm's row 2 of 0x080EC2AC, the shot's row 0x0C of 0x080EC2B3)
-- firing EXE5's GigaCan shot (objects/projectile: a blast, no burst), the cannon and the navi leaving
-- afterimages as it charges (a third, Hub Style's shade, in Hub Style: lib/cannon).
local gigacan3: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 600,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- EXE5's recipes (written by tools/exe5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 2, sequence = { recipe_attck_10, recipe_m_cannon, recipe_m_cannon }, operation_battle_only = true },
        { order = 38, code_run = { chip = recipe_m_cannon, count = 3 } },
    },
    action = shooter.action {
        arm = GIGACAN3_ARM,
        shot = variants.gigacan,
        afterimages = true,
    },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { gigacan1 = gigacan1, gigacan2 = gigacan2, gigacan3 = gigacan3 }
