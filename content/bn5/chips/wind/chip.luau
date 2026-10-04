--!strict
-- Wind (BN5 chip 0x053): a dimming that places a fan in front of the user (./controller, the dimming handler's
-- subtype 9) that blows a gust along the enemy's rows every 12 ticks for 24 seconds, pushing away from the enemy's
-- front column (./fan, the record's first parameter 0).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("./controller")
local fan = require("./fan")

return define.chip {
    id = "bn5:wind",
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
            id = "bn5:wind",
            class = "field_object",
            version = 3,
            ai_index = 0x21,
            look = { sprite = fan.SPRITE, anim = 0, palette = 0, shadow = false },
        },
    }),
}
