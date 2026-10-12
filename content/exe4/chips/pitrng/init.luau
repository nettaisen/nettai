--!strict
-- PitRng1, PitRng2 and PitRng3 (EXE4 chips 0x14C to 0x14E; their records at 0x080197EC + n * 0x2C): the Program
-- Advances of three Boomer1s, Boomer2s or Boomer3s of consecutive codes (the recipe table, the pointer list at
-- 0x080212B0). The spawners' action 0x20 (lib/spawners), its variant 0x0D (0x080E9A36): a thrower (effect object
-- #0x6C, 0x080E999C) of the user's side, with the attack's element and damage word, throws three of the Boomers'
-- boomerangs (./boomer/boomerang, its row 5) from its side's back column, 16 pixels up, one every 16 ticks: the first
-- along row 3, turning at the third column from the far edge, the second along row 1, turning at the second, the third
-- along row 3, turning at the far column; each a strong one (hit modifier 3).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local boomerang = require("./boomer/boomerang")
local boomers = require("./boomer")
local common = require("@exelib/boomer/boomerang")
local spawners = require("../lib/spawners")

-- The thrower's boomerangs, and the ticks it waits after each (0x080E99E4, 0x080E99F8).
local THROWS, WAIT = 3, 15
local HEIGHT = 0x10_0000

-- Its phase once set up.
local THROW = 0x4

-- 0x080E99EE: a boomerang when its timer runs out (at once the first time), by how many it has left: its row 3 with
-- an odd number left, row 1 with an even one; the column it turns at one less than the number left (the boomerang's
-- second parameter: 2, 1, 0); gone after the third.
local function tick(me: Object)
    local t = me.timer - 1
    if t >= 0 then
        me.timer = t
        return
    end
    me.timer = WAIT
    local left = me.timer2
    local x = me.alliance * 5 + 1
    local y = bit32.band(left, 1) * 2 + 1
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    common.spawn(boomerang.kind, me, x, y, me.element, HEIGHT, { variant = boomerang.boomer, column = left - 1, strong = true }, damage)
    left -= 1
    me.timer2 = left
    if left <= 0 then
        me:free()
    end
end

local THROWER = new.kind {
    id = "pitrng/thrower",
    pool = "effect",
    -- 0x080E999C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E99BC: its first tick runs its first update too, the setup (0x080E99E4: phase 0, which it leaves).
            me:set_lifecycle("update")
            me.timer2 = THROWS
            me.phase, me.phase_init = THROW, 0
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- 0x080E9A36: the spawner: the thrower, of the user's side, with the attack's element and damage word. Its position
-- is the registers (the user's row, the element, the user's height), which nothing reads; it has no panel.
local function spawn(user: Object, element: number, damage: number)
    local o = battle.spawn(THROWER, Vec3.new(user.panel_y, element, user.pos.z))
    if o then
        o.element = element
        o:set_damage_word(damage)
        o.alliance = user.alliance
    end
end

local pitrng1: Chip = {
    action = spawners.action { id = "pitrng1/action", spawn = spawn },
    codes = { "A", "B", "C", "*" },
    element = "wood",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 200,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 23, code_run = { chip = boomers.boomer1, count = 3 } } },
    auto_battle = { class = "into_reach", range = "ahead_then_column" },
}

local pitrng2: Chip = {
    action = spawners.action { id = "pitrng2/action", spawn = spawn },
    codes = { "A", "B", "C", "*" },
    element = "wood",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 250,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 24, code_run = { chip = boomers.boomer2, count = 3 } } },
    auto_battle = { class = "into_reach", range = "ahead_then_column" },
}

local pitrng3: Chip = {
    action = spawners.action { id = "pitrng3/action", spawn = spawn },
    codes = { "A", "B", "C", "*" },
    element = "wood",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 300,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = { { order = 25, code_run = { chip = boomers.boomer3, count = 3 } } },
    auto_battle = { class = "into_reach", range = "ahead_then_column" },
}

return { pitrng1 = pitrng1, pitrng2 = pitrng2, pitrng3 = pitrng3 }
