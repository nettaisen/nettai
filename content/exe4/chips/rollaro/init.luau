--!strict
-- RollAro1 to RollAro3: the dimming chips' variant 0x35, EXE4's own: a controller (effect object #0x63, 0x080E8FA0:
-- run as the navi chips', lib/navi_chips' long course, 0x080E8FE8) on which the user warps out (30 ticks), Roll comes
-- and shoots an arrow down her row (./roll), 30 ticks pass and the user warps back in (30 ticks); each chip its own
-- controller (each chip's own comment above its definition).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi_chips = require("../lib/navi_chips")
local roll = require("@self/roll")

local function controller(id: string): (user: Object, spec: DimmingChipSpec) -> Object?
    return navi_chips.controller({ id = id, name = "RollAro", spawn = roll.spawn, steps = navi_chips.LONG }).hook
end

-- RollAro1 (EXE4 chip 0x097).
local rollaro1: Chip = {
    codes = { "A", "F", "T" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 32,
    damage = 50,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("rollaro1/controller"),
}

-- RollAro2 (EXE4 chip 0x098).
local rollaro2: Chip = {
    codes = { "D", "R", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 38,
    damage = 70,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("rollaro2/controller"),
}

-- RollAro3 (EXE4 chip 0x099).
local rollaro3: Chip = {
    codes = { "Q", "Y", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 44,
    damage = 90,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("rollaro3/controller"),
}

return { rollaro1 = rollaro1, rollaro2 = rollaro2, rollaro3 = rollaro3 }
