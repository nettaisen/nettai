--!strict
-- GigaCan1-3: the Program Advances of three Cannons, HiCannons or
-- M-Cannons of one code. The cannon of the chip they're made of fires a
-- shot that bursts over the 3x3 around what it hits (and over the last two
-- columns when it leaves the field), and the cannon and the navi leave
-- afterimages as it charges (the original's action 0x14 with subtypes 4 to
-- 6; lib/cannon.luau).

local cannon = require("../lib/cannon")
local variants = require("../objects/projectile/variants")
local cannons = require("./cannon")
local lockon = require("../rules/lockon")

local function giga(id: string, arm: AttachmentLook): Action
    return cannon.action { id = id, arm = arm, shot = variants.gigacan, afterimages = true }
end

local gigacan1 = define.chip {
    id = "gigacan1",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 300,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "library" },
    beast = { lockon = lockon.cannon },
    library = { number = 320, index = 1, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 13, code_run = { chip = cannons.cannon, count = 3 } } },
    action = giga("gigacan1/action", cannon.arms.cannon),
}

local gigacan2 = define.chip {
    id = "gigacan2",
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
    beast = { lockon = lockon.cannon },
    library = { number = 321, index = 2, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 14, code_run = { chip = cannons.hicannon, count = 3 } } },
    action = giga("gigacan2/action", cannon.arms.hicannon),
}

local gigacan3 = define.chip {
    id = "gigacan3",
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
    beast = { lockon = lockon.cannon },
    library = { number = 322, index = 3, sort = 0 },
    slot_in_limit = 1,
    program_advances = { { order = 15, code_run = { chip = cannons.m_cannon, count = 3 } } },
    action = giga("gigacan3/action", cannon.arms.m_cannon),
}

return { gigacan1 = gigacan1, gigacan2 = gigacan2, gigacan3 = gigacan3 }
