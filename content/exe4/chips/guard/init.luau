--!strict
-- Guard1, Guard2 and Guard3 (EXE4 chips 0x058 to 0x05A; their records at 0x080197EC + n * 0x2C): EXE4's guard
-- (lib/guard, action 0x25, its variants 0 to 2): the navi holds up a shield (its look row 0: animation 0) for 60 ticks;
-- the first hit it turns aside sends a shock wave back along the row from the panel ahead with the chip's damage.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local guard = require("../lib/guard")
local shield = require("../objects/shield")
local wave = require("@self/wave")

-- Their counter (0x080ECD98): a shock wave (0x080CFD2A) of row 6 for all three (0x080ECE10's first byte: animation 0,
-- 10 ticks a panel).
local WAVE = wave.variant { anim = 0, ticks = 0x0A }
local function counter(me: Object, x: number, y: number, damage: number)
    wave.spawn(me, x, y, WAVE, damage)
end

local TICKS = 0x3C
local SHIELD = shield.look { anim = 0 }

local guard1: Chip = {
    action = guard.action { id = "guard1/action", ticks = TICKS, shield = SHIELD, counter = counter },
    codes = { "A", "L", "V", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 50,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local guard2: Chip = {
    action = guard.action { id = "guard2/action", ticks = TICKS, shield = SHIELD, counter = counter },
    codes = { "C", "G", "P", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 10,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local guard3: Chip = {
    action = guard.action { id = "guard3/action", ticks = TICKS, shield = SHIELD, counter = counter },
    codes = { "F", "R", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 16,
    damage = 110,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { guard1 = guard1, guard2 = guard2, guard3 = guard3 }
