--!strict
-- Sword, WideSwrd, LongSwrd, WideBlde and LongBlde (EXE4 chips 0x030 to 0x034; their records at 0x080197EC + n *
-- 0x2C): the swords' action 0x0A (lib/swords), its variants 0 to 4, each by its rows: the region word (0x080EB7B0:
-- 0x0705FF01 the panel ahead, 0x0705FF04 the column ahead, 0x0705FF02 the panel ahead and the one past it; hits
-- navis, row 7 its own), the hit modifier (0x080EB7F1: 3), the slash's effect (0x080EB7E4: rows 0x19, 0x17, 0x18,
-- 0x1A, 0x1B), the swing's sound (0x8E).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local HIT = { hits = collision.hits_navis, is = collision.slash }

local function slash(id: string, region: Region, effect: Effect): Action
    return swords.slash {
        id = id,
        blade = swords.blade,
        hit = { region = region, collision = HIT, hit_mod = 3 },
        effect = effect,
        sound = swords.swing,
    }
end

local sword: Chip = {
    action = slash("sword/action", regions.single, swords.effects.slash),
    codes = { "E", "L", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 9,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local wideswrd: Chip = {
    action = slash("wideswrd/action", regions.wide, swords.effects.wide),
    codes = { "E", "L", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 1,
    mb = 16,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local longswrd: Chip = {
    action = slash("longswrd/action", regions.long, swords.effects.long),
    codes = { "E", "L", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local wideblde: Chip = {
    action = slash("wideblde/action", regions.wide, swords.effects.wide_blade),
    codes = { "C", "K", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 38,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local longblde: Chip = {
    action = slash("longblde/action", regions.long, swords.effects.long_blade),
    codes = { "G", "R", "S" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 40,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { sword = sword, wideswrd = wideswrd, longswrd = longswrd, wideblde = wideblde, longblde = longblde }
