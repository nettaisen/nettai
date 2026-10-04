--!strict
-- WideSht1, WideSht2 and WideSht3: the widesht series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local wave = require("./variants")
local widesht = require("./wave")

-- WideSht1 (BN5 chip 0x009): the navi raises its wide shooter and sends a wave from the panel ahead over a column
-- of three: the shared WideSht (@exelib/widesht, the original's action 0x30, BN5's 0x25; the same code but for the
-- shooter's attachment row, BN5's 0x11 for BN6's 0x23, and the wave's height and sprite) with BN5's shooter (0c-2a,
-- the same place), its wave at the floor (BN6's at 16 pixels) and BN5's wide wave (chips/widesht).
local widesht1 = define.chip {
    id = "bn5:widesht1",
    codes = { "L", "M", "N" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 70,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = widesht.action { wave = wave.wide, waves = 1, z = 0 },
}

-- WideSht2 (BN5 chip 0x00a): the navi raises its wide shooter and sends a wave from the panel ahead over a column
-- of three: the shared WideSht (@exelib/widesht, the original's action 0x30, BN5's 0x25; the same code but for the
-- shooter's attachment row, BN5's 0x11 for BN6's 0x23, and the wave's height and sprite) with BN5's shooter (0c-2a,
-- the same place), its wave at the floor (BN6's at 16 pixels) and BN5's wide wave of kind 4 (chips/widesht).
local widesht2 = define.chip {
    id = "bn5:widesht2",
    codes = { "E", "F", "G" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 90,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = widesht.action { wave = wave.wide2, waves = 1, z = 0 },
}

-- WideSht3 (BN5 chip 0x00b): the navi raises its wide shooter and sends a wave from the panel ahead over a column
-- of three: the shared WideSht (@exelib/widesht, the original's action 0x30, BN5's 0x25; the same code but for the
-- shooter's attachment row, BN5's 0x11 for BN6's 0x23, and the wave's height and sprite) with BN5's shooter (0c-2a,
-- the same place), its wave at the floor (BN6's at 16 pixels) and BN5's wide wave of kind 5 (chips/widesht).
local widesht3 = define.chip {
    id = "bn5:widesht3",
    codes = { "S", "T", "U" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 110,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = widesht.action { wave = wave.wide3, waves = 1, z = 0 },
}

return { widesht1 = widesht1, widesht2 = widesht2, widesht3 = widesht3 }
