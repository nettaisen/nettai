--!strict
-- Vulcan1, Vulcan2 and Vulcan3 (EXE4 chips 0x00F to 0x011; their records at 0x080197EC + n * 0x2C): the vulcans'
-- action 0x1F (./vulcan), its variants 0 to 2: the gun (attachment row 13) fires 3, 5 or 7 shots (0x080EC934), bullet
-- row 12 (their parameter +0x10).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bullet = require("../objects/bullet")
local vulcan = require("@self/vulcan")

local function fire(id: string, shots: number): Action
    return vulcan.action { id = id, gun = vulcan.guns.vulcan, shots = shots, shot = bullet.variants.vulcan }
end

local vulcan1: Chip = {
    action = fire("vulcan1/action", 3),
    codes = { "E", "S", "V", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local vulcan2: Chip = {
    action = fire("vulcan2/action", 5),
    codes = { "B", "H", "O" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 18,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local vulcan3: Chip = {
    action = fire("vulcan3/action", 7),
    codes = { "O", "T", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { vulcan1 = vulcan1, vulcan2 = vulcan2, vulcan3 = vulcan3 }
