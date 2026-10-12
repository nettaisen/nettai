--!strict
-- MagBolt1, MagBolt2 and MagBolt3 (EXE4 chips 0x023 to 0x025; their records at 0x080197EC + n * 0x2C): the magnets'
-- action 0x59 (./action), its variants 0 to 2: the field on the panel ahead of the navi, the pull over the six panels
-- ahead (region 8), no paralysis (their parameters' first byte, 0).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local magbolt = require("@self/action")
local regions = require("@exelib/regions")

local function action(id: string): Action
    return magbolt.action {
        id = id,
        rows = { 0 },
        paralyzes = false,
        pull = function(_me: Object): Region
            return regions.row
        end,
    }
end

local magbolt1: Chip = {
    action = action("magbolt1/action"),
    codes = { "B", "C", "D", "*" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 18,
    damage = 90,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

local magbolt2: Chip = {
    action = action("magbolt2/action"),
    codes = { "E", "F", "G" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 28,
    damage = 110,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

local magbolt3: Chip = {
    action = action("magbolt3/action"),
    codes = { "A", "B", "C" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 38,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { magbolt1 = magbolt1, magbolt2 = magbolt2, magbolt3 = magbolt3 }
