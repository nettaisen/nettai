--!strict
-- GutPnch1 to GutPnch3: the gutpnch series, the dimming chips' variant 0x36: a controller (effect object #0x64,
-- 0x080E90CC: run as the navi chips', lib/navi_chips) on which the user warps out (30 ticks), GutsMan comes and punches
-- the panel ahead (./guts), 30 ticks pass and the user warps back in with the appearing sound (0x080E91AC, 30 ticks);
-- each chip its own controller (each chip's own comment above its definition).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local guts = require("@self/guts")
local navi_chips = require("../lib/navi_chips")

-- 0x080E9124's phases: the warp out and 30 ticks, GutsMan, 30 ticks, the warp back in with its sound and 30 ticks.
local COURSE = {
    { wait = 0x1E, warp = "out" },
    "navi",
    { wait = 0x1E },
    { wait = 0x1E, warp = "in", sound = asset.sound("appear") },
} :: { NaviChipStep }

local function controller(id: string): (user: Object, spec: DimmingChipSpec) -> Object?
    return navi_chips.controller({ id = id, name = "GutPnch", spawn = guts.spawn, steps = COURSE }).hook
end

-- GutPnch1 (EXE4 chip 0x09A).
local gutpnch1: Chip = {
    codes = { "B", "E", "N" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 22,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("gutpnch1/controller"),
}

-- GutPnch2 (EXE4 chip 0x09B).
local gutpnch2: Chip = {
    codes = { "G", "P", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 36,
    damage = 140,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("gutpnch2/controller"),
}

-- GutPnch3 (EXE4 chip 0x09C).
local gutpnch3: Chip = {
    codes = { "M", "Q", "T" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 50,
    damage = 180,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("gutpnch3/controller"),
}

return { gutpnch1 = gutpnch1, gutpnch2 = gutpnch2, gutpnch3 = gutpnch3 }
