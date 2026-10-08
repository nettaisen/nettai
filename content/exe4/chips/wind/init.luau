--!strict
-- Wind and Fan: the wind series, the dimming chips' variant 0x0F (each chip's own comment above its definition).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local controller = require("@self/controller")
local fan = require("@self/fan")

-- Wind (EXE4 chip 0x053): a fan in front of the user (./controller) that blows a gust along the enemy's rows every 12
-- ticks for 24 seconds, pushing away from the enemy's front column (./fan; 0x080D6480's first row: rising after a
-- tick, 100 HP, its palette 0).
local wind: Chip = {
    codes = { "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 1,
    mb = 10,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = controller.hook(fan.variant {
        palette = 0,
        pulls = false,
        wait = 1,
        hp = 100,
    }),
}

-- Fan (EXE4 chip 0x054): a fan in front of the user (./controller) that blows a gust along the enemy's rows every 12
-- ticks for 24 seconds, pulling toward the enemy's front column from the far one (./fan; 0x080D6480's second row:
-- rising after a tick, 100 HP, its palette 1).
local fan_chip: Chip = {
    codes = { "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 1,
    mb = 10,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = controller.hook(fan.variant {
        palette = 1,
        pulls = true,
        wait = 1,
        hp = 100,
    }),
}

return { wind = wind, fan = fan_chip }
