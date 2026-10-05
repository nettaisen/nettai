--!strict
-- WideSht1, WideSht2 and WideSht3: the widesht series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local wave = require("@self/variants")
local widesht = require("@self/wave")

-- WideSht1 (EXE5 chip 0x009): the navi raises its wide shooter and sends a wave from the panel ahead over a column
-- of three: the shared WideSht (@exelib/widesht, the original's action 0x30, EXE5's 0x25; the same code but for the
-- shooter's attachment row, EXE5's 0x11 for EXE6's 0x23, and the wave's height and sprite) with EXE5's shooter (0c-2a,
-- the same place), its wave at the floor (EXE6's at 16 pixels) and EXE5's wide wave (chips/widesht).
local widesht1 = define.chip {
    id = "widesht1",
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

-- WideSht2 (EXE5 chip 0x00a): the navi raises its wide shooter and sends a wave from the panel ahead over a column
-- of three: the shared WideSht (@exelib/widesht, the original's action 0x30, EXE5's 0x25; the same code but for the
-- shooter's attachment row, EXE5's 0x11 for EXE6's 0x23, and the wave's height and sprite) with EXE5's shooter (0c-2a,
-- the same place), its wave at the floor (EXE6's at 16 pixels) and EXE5's wide wave of kind 4 (chips/widesht).
local widesht2 = define.chip {
    id = "widesht2",
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

-- WideSht3 (EXE5 chip 0x00b): the navi raises its wide shooter and sends a wave from the panel ahead over a column
-- of three: the shared WideSht (@exelib/widesht, the original's action 0x30, EXE5's 0x25; the same code but for the
-- shooter's attachment row, EXE5's 0x11 for EXE6's 0x23, and the wave's height and sprite) with EXE5's shooter (0c-2a,
-- the same place), its wave at the floor (EXE6's at 16 pixels) and EXE5's wide wave of kind 5 (chips/widesht).
local widesht3 = define.chip {
    id = "widesht3",
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
