--!strict
-- GigaCan1-3: the Program Advances of three Cannons, HiCannons or
-- M-Cannons of one code. The cannon of the chip they're made of fires a
-- shot that bursts over the 3x3 around what it hits (and over the last two
-- columns when it leaves the field), and the cannon and the navi leave
-- afterimages as it charges (the original's action 0x14 with subtypes 4 to
-- 6; lib/cannon.luau).

local cannon = require("../lib/cannon")
local variants = require("../objects/projectile/variants")
local cannon_chips = require("./cannon")
local lockon = require("../rules/lockon")

local function giga(id: string, arm: AttachmentLook): Action
    return cannon.action { id = id, arm = arm, shot = variants.gigacan, afterimages = true }
end

local gigacan1: Chip = {
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
    slot_in_limit = 1,
    program_advances = { { order = 13, code_run = { chip = cannon_chips.cannon, count = 3 } } },
    action = giga("gigacan1/action", cannon.arms.cannon),
}

local gigacan2: Chip = {
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
    slot_in_limit = 1,
    program_advances = { { order = 14, code_run = { chip = cannon_chips.hicannon, count = 3 } } },
    action = giga("gigacan2/action", cannon.arms.hicannon),
}

local gigacan3: Chip = {
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
    slot_in_limit = 1,
    program_advances = { { order = 15, code_run = { chip = cannon_chips["m-cannon"], count = 3 } } },
    action = giga("gigacan3/action", cannon.arms.m_cannon),
}

return { gigacan1 = gigacan1, gigacan2 = gigacan2, gigacan3 = gigacan3 }
