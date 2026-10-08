--!strict
-- GunSol1, GunSol2, GunSol3 and GunSolEX (EXE4 chips 0x020 to 0x022 and 0x0DB; their records at 0x080197EC + n * 0x2C):
-- EXE4's GunSol (./gunsol, action 0x58, its variants 0 to 3): a sun beam that drains the column two panels ahead while
-- A is held, more in the sun, for 60, 90 or 120 ticks; GunSolEX's the column past it too, for 120.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local gunsol = require("@self/gunsol")
local regions = require("@exelib/regions")

-- GunSol1 to 3's beam (0x080E7130's first sprite) in the shade and in the sun, and GunSolEX's (its second).
local BEAM: SunBeamLook = { sprite = asset.sprite("sun-beam"), palette = 0 }
local BEAM_IN_SUN: SunBeamLook = { sprite = asset.sprite("sun-beam"), palette = 2 }
local WIDE_BEAM: SunBeamLook = { sprite = asset.sprite("sun-beam-ex"), palette = 0 }
local WIDE_BEAM_IN_SUN: SunBeamLook = { sprite = asset.sprite("sun-beam-ex"), palette = 3 }

local gunsol1: Chip = {
    action = gunsol.action {
        id = "gunsol1/action",
        gun = gunsol.gun(0),
        firing_ticks = 60,
        beam = BEAM,
        beam_in_sun = BEAM_IN_SUN,
        region = regions.wide,
        columns = 1,
    },
    codes = { "A", "G", "M", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 15,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

local gunsol2: Chip = {
    action = gunsol.action {
        id = "gunsol2/action",
        gun = gunsol.gun(3),
        firing_ticks = 90,
        beam = BEAM,
        beam_in_sun = BEAM_IN_SUN,
        region = regions.wide,
        columns = 1,
    },
    codes = { "B", "G", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

local gunsol3: Chip = {
    action = gunsol.action {
        id = "gunsol3/action",
        gun = gunsol.gun(6),
        firing_ticks = 120,
        beam = BEAM,
        beam_in_sun = BEAM_IN_SUN,
        region = regions.wide,
        columns = 1,
    },
    codes = { "C", "G", "T" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 35,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

local gunsolex: Chip = {
    action = gunsol.action {
        id = "gunsolex/action",
        gun = gunsol.gun(9),
        firing_ticks = 120,
        beam = WIDE_BEAM,
        beam_in_sun = WIDE_BEAM_IN_SUN,
        region = regions.wide_long,
        columns = 2,
    },
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 80,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
}

return { gunsol1 = gunsol1, gunsol2 = gunsol2, gunsol3 = gunsol3, gunsolex = gunsolex }
