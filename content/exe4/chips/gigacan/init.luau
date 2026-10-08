--!strict
-- GigaCan1, GigaCan2 and GigaCan3 (EXE4 chips 0x143 to 0x145; their records at 0x080197EC + n * 0x2C): the Program
-- Advances of three Cannons, HiCannons or M-Cannons of consecutive codes (the recipe table's code runs, the pointer
-- list at 0x080212B0). The cannons' action 0x0B (lib/cannon), its variants 4 to 6: the cannon in palettes 0 to 2
-- (attachment rows 0 to 2 by 0x080EB9B4, the Cannons'), the GigaCan's shot (projectile row 12 by 0x080EB9BB), and the
-- GigaCan's afterimages (0x080EB8BE).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local cannon_lib = require("../lib/cannon")
local cannons = require("./cannon")
local collision = require("../rules/collision")
local projectile = require("../objects/projectile")
local sparks = require("../lib/sparks")

-- Their cannons (attachment rows 0 to 2: 0c-01 in palettes 0 to 2).
local GIGACAN1_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 0 }
local GIGACAN2_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 1 }
local GIGACAN3_ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 2 }

-- The GigaCan's shot (projectile row 12, action 0x0B's variants 4 to 6): it knocks back (hit modifier 9), with the
-- blast spark.
local SHOT = projectile.variant {
    id = "gigacan/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 9,
    element = "null",
    spark = sparks.blast,
}

local gigacan1: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 300,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 14, code_run = { chip = cannons.cannon, count = 3 } } },
    action = cannon_lib.action { id = "gigacan1/action", arm = GIGACAN1_ARM, shot = SHOT, afterimages = true },
}

local gigacan2: Chip = {
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
    program_advances = { { order = 15, code_run = { chip = cannons.hicannon, count = 3 } } },
    action = cannon_lib.action { id = "gigacan2/action", arm = GIGACAN2_ARM, shot = SHOT, afterimages = true },
}

local gigacan3: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 500,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 16, code_run = { chip = cannons["m-cannon"], count = 3 } } },
    action = cannon_lib.action { id = "gigacan3/action", arm = GIGACAN3_ARM, shot = SHOT, afterimages = true },
}

return { gigacan1 = gigacan1, gigacan2 = gigacan2, gigacan3 = gigacan3 }
