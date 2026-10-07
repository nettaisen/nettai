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

-- Their slashes (the effect table 0x080E1698's rows 0x19, 0x17, 0x18, 0x1A and 0x1B: the slash sprite 0c-14): one
-- panel's, a column's, two panels', and the wide and long ones in palette 5.
local SLASH = asset.sprite("sword-slash")
local SWORD_SLASH = new.effect { sprite = SLASH, anim = 2, palette = 0 }
local WIDE_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 0 }
local LONG_SLASH = new.effect { sprite = SLASH, anim = 1, palette = 0 }
local WIDE_BLADE_SLASH = new.effect { sprite = SLASH, anim = 0, palette = 5 }
local LONG_BLADE_SLASH = new.effect { sprite = SLASH, anim = 1, palette = 5 }

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
    action = slash("sword/action", regions.single, SWORD_SLASH),
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
    action = slash("wideswrd/action", regions.wide, WIDE_SLASH),
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
    action = slash("longswrd/action", regions.long, LONG_SLASH),
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
    action = slash("wideblde/action", regions.wide, WIDE_BLADE_SLASH),
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
    action = slash("longblde/action", regions.long, LONG_BLADE_SLASH),
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
