--!strict
-- Wind and Fan: the wind series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")
local fan = require("@self/fan")

-- Wind (EXE5 chip 0x053): a dimming that places a fan in front of the user (./controller, the dimming handler's
-- subtype 9) that blows a gust along the enemy's rows every 12 ticks for 24 seconds, pushing away from the enemy's
-- front column (./fan, the record's first parameter 0).
local wind = define.chip {
    id = "wind",
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
    extra_flags = { 128 },
    slot_in_limit = 3,
    -- 0x080CE72C's first row; NameID 0xD6: actor record 0xD6 (0x08014C94: version 3, AI index 0x21).
    dimming = controller.hook(fan.variant {
        palette = 0,
        pulls = false,
        wait = 1,
        hp = 40,
        identity = define.identity {
            id = "wind",
            class = "field_object",
            version = 3,
            ai_index = 0x21,
            look = { sprite = fan.SPRITE, anim = 0, palette = 0, shadow = false },
        },
    }),
}

-- Fan (EXE5 chip 0x054): a dimming that places a fan in front of the user (./controller, the dimming handler's
-- subtype 9) that blows a gust along the enemy's rows every 12 ticks for 24 seconds, pulling from the far column
-- (./fan, the record's first parameter 1).
local fan_chip = define.chip {
    id = "fan",
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
    extra_flags = { 128 },
    slot_in_limit = 3,
    -- 0x080CE72C's second row; NameID 0xD7: actor record 0xD7 (0x08014C94: version 4, AI index 0x21).
    dimming = controller.hook(fan.variant {
        palette = 1,
        pulls = true,
        wait = 1,
        hp = 40,
        identity = define.identity {
            id = "fan",
            class = "field_object",
            version = 4,
            ai_index = 0x21,
            look = { sprite = fan.SPRITE, anim = 0, palette = 1, shadow = false },
        },
    }),
}

return { wind = wind, fan = fan_chip }
