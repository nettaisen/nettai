--!strict
-- AquaUp1, AquaUp2 and AquaUp3 (EXE4 chips 0x0B5 to 0x0B7; their records at 0x080197EC + n * 0x2C): the spawners'
-- action 0x20 (lib/spawners), its variant 0x0C, their parameter 0: the towers' controller (lib/towers) raises an aqua
-- tower from each cracked panel of the other side's holding its navi, a column at a time.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local spawners = require("../lib/spawners")
local tower = require("../objects/tower")
local towers = require("../lib/towers")

-- The aqua tower (the towers' row 0): 0c-25, animations 0, 1 and 2 for 12, 4 and 9 ticks; hit modifier 3; sound 0x120.
local AQUA_TOWER = tower.make {
    id = "aquaup/tower",
    sprite = asset.sprite("geyser"),
    steps = { { anim = 0, ticks = 0x0B }, { anim = 1, ticks = 3 }, { anim = 2, ticks = 8 } },
    hit_mod = 3,
    sound = asset.sound("aqua-tower"),
}

-- 0x080DF8A4: a cracked panel of the other side's holding its navi: an aqua tower, and the panel broken if nothing
-- stands on it (0x08009BAC: something always does).
local function strike(me: Object, x: number, y: number): boolean
    if not towers.enemy_panel(me, x, y, "cracked") then
        return false
    end
    AQUA_TOWER(me, x, y, me.element, towers.damage_word(me))
    field.break_empty(x, y)
    return true
end

local RAISE = towers.controller { id = "aquaup/controller", strike = strike }

local aquaup1: Chip = {
    action = spawners.action { id = "aquaup1/action", spawn = RAISE },
    codes = { "B", "S", "Z" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 110,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

local aquaup2: Chip = {
    action = spawners.action { id = "aquaup2/action", spawn = RAISE },
    codes = { "I", "P", "U" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 28,
    damage = 140,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

local aquaup3: Chip = {
    action = spawners.action { id = "aquaup3/action", spawn = RAISE },
    codes = { "F", "K", "W" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 34,
    damage = 170,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

return { aquaup1 = aquaup1, aquaup2 = aquaup2, aquaup3 = aquaup3 }
