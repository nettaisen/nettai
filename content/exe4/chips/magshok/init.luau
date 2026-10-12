--!strict
-- MagShok1, MagShok2 and MagShok3 (EXE4 chips 0x155 to 0x157; their records at 0x080197EC + n * 0x2C): the Program
-- Advances of three MagBolt1s, MagBolt2s or MagBolt3s of consecutive codes (the recipe table, the pointer list at
-- 0x080212B0). The magnets' action 0x59 (./magbolt/action), its variant 3: MagBolt1's magnet (attachment row 0x23),
-- fields on the panel ahead and the two beside it, paralyzing (their parameters' first byte, 1), and the pull over
-- the other side's area (region 0x82 from side 0, 0x81 from side 1: 0x080EE148).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local magbolt = require("./magbolt/action")
local magbolts = require("./magbolt")
local regions = require("@exelib/regions")

local function action(id: string): Action
    return magbolt.action {
        id = id,
        rows = { 0, -1, 1 },
        paralyzes = true,
        pull = function(me: Object): Region
            return regions.area[1 - me.alliance]
        end,
    }
end

local magshok1: Chip = {
    action = action("magshok1/action"),
    codes = { "A", "B", "C", "*" },
    element = "elec",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 200,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 32, code_run = { chip = magbolts.magbolt1, count = 3 } } },
}

local magshok2: Chip = {
    action = action("magshok2/action"),
    codes = { "A", "B", "C", "*" },
    element = "elec",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 250,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 33, code_run = { chip = magbolts.magbolt2, count = 3 } } },
}

local magshok3: Chip = {
    action = action("magshok3/action"),
    codes = { "A", "B", "C", "*" },
    element = "elec",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 300,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 34, code_run = { chip = magbolts.magbolt3, count = 3 } } },
}

return { magshok1 = magshok1, magshok2 = magshok2, magshok3 = magshok3 }
