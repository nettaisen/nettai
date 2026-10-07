--!strict
-- GreenWd1, GreenWd2 and GreenWd3 (EXE4 chips 0x0B8 to 0x0BA; their records at 0x080197EC + n * 0x2C): the spawners'
-- action 0x20 (lib/spawners), its variant 0x0C, their parameter 1: the towers' controller (lib/towers) raises a wood
-- tower from each grass panel of the other side's holding its navi, a column at a time.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local spawners = require("../lib/spawners")
local tower = require("../objects/tower")
local towers = require("../lib/towers")

-- The wood tower (the towers' row 1): 0c-0b, animation 0 for 12 ticks and 4 more, then 1 for 10; hit modifier 1;
-- sound 0x13E.
local WOOD_TOWER = tower.make {
    id = "greenwd/tower",
    sprite = asset.sprite("wood-tower"),
    steps = { { anim = 0, ticks = 0x0B }, { ticks = 3 }, { anim = 1, ticks = 9 } },
    hit_mod = 1,
    sound = asset.sound("wood-tower"),
}

-- 0x080DF908: a grass panel of the other side's holding its navi: a wood tower.
local function strike(me: Object, x: number, y: number): boolean
    if not towers.enemy_panel(me, x, y, "grass") then
        return false
    end
    WOOD_TOWER(me, x, y, me.element, towers.damage_word(me))
    return true
end

local RAISE = towers.controller { id = "greenwd/controller", strike = strike }

local greenwd1: Chip = {
    action = spawners.action { id = "greenwd1/action", spawn = RAISE },
    codes = { "G", "J", "Y" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 1,
    mb = 35,
    damage = 120,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

local greenwd2: Chip = {
    action = spawners.action { id = "greenwd2/action", spawn = RAISE },
    codes = { "D", "K", "P" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 140,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

local greenwd3: Chip = {
    action = spawners.action { id = "greenwd3/action", spawn = RAISE },
    codes = { "E", "N", "P" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 3,
    mb = 45,
    damage = 160,
    hit_param = 30,
    lockout = 45,
    flags = { "has_damage", "library" },
}

return { greenwd1 = greenwd1, greenwd2 = greenwd2, greenwd3 = greenwd3 }
