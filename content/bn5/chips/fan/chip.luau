--!strict
-- Fan (BN5 chip 0x054): a dimming that places a fan in front of the user (../wind/controller, the dimming handler's
-- subtype 9) that blows a gust along the enemy's rows every 12 ticks for 24 seconds, pulling from the far column
-- (../wind/fan, the record's first parameter 1).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("../wind/controller")
local fan = require("../wind/fan")

return define.chip {
    id = "bn5:fan",
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
            id = "bn5:fan",
            class = "field_object",
            version = 4,
            ai_index = 0x21,
            look = { sprite = fan.SPRITE, anim = 0, palette = 1, shadow = false },
        },
    }),
}
