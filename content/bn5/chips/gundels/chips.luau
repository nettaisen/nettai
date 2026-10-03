--!strict
-- GunDelS1, GunDelS2 and GunDelS3: the gundels series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local gundels = require("./gundels")

-- GunDelS1 (BN5 chip 0x020): a sun beam that drains the column two panels ahead while A is held, more in the sun, for
-- at most 60 ticks: BN5's GunDelSol (chips/gundels/gundels; BN6's code but for holding A, its collision row and its firing
-- ticks).
local gundels1 = define.chip {
    id = "bn5:gundels1",
    codes = { "A", "S", "U", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 15,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "light",
    action = gundels.action {
        id = "bn5:gundels1/action",
        gun = gundels.gun(0),
        firing_ticks = 60,
        beam = gundels.beams.plain,
        beam_in_sun = gundels.beams.plain_in_sun,
    },
}

-- GunDelS2 (BN5 chip 0x021): a sun beam that drains the column two panels ahead while A is held, more in the sun, for
-- at most 90 ticks: BN5's GunDelSol (chips/gundels/gundels; BN6's code but for holding A, its collision row and its firing
-- ticks).
local gundels2 = define.chip {
    id = "bn5:gundels2",
    codes = { "G", "U", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "light",
    action = gundels.action {
        id = "bn5:gundels2/action",
        gun = gundels.gun(3),
        firing_ticks = 90,
        beam = gundels.beams.plain,
        beam_in_sun = gundels.beams.plain_in_sun,
    },
}

-- GunDelS3 (BN5 chip 0x022): a sun beam that drains the column two panels ahead while A is held, more in the sun, for
-- at most 120 ticks: BN5's GunDelSol (chips/gundels/gundels; BN6's code but for holding A, its collision row and its firing
-- ticks).
local gundels3 = define.chip {
    id = "bn5:gundels3",
    codes = { "D", "M", "O" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 35,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "light",
    action = gundels.action {
        id = "bn5:gundels3/action",
        gun = gundels.gun(6),
        firing_ticks = 120,
        beam = gundels.beams.plain,
        beam_in_sun = gundels.beams.plain_in_sun,
    },
}

-- GunDelEX (BN5 chip 0x0db): a wide sun beam that drains the column two panels ahead and the one past it while A is
-- held, more in the sun, for at most 120 ticks: BN5's GunDelSol (chips/gundels/gundels, action 0x2F with subtype 3: the
-- gun's attachment row 0x16, palette 9; the firing ticks' fourth, 120; the wide beams).
local gundelex = define.chip {
    id = "bn5:gundelex",
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 80,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    action = gundels.action {
        id = "bn5:gundelex/action",
        gun = gundels.gun(9),
        firing_ticks = 120,
        beam = gundels.beams.wide,
        beam_in_sun = gundels.beams.wide_in_sun,
        wide = true,
    },
}

return { gundels1 = gundels1, gundels2 = gundels2, gundels3 = gundels3, gundelex = gundelex }
