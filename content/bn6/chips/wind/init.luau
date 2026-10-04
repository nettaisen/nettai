--!strict
-- Wind and Fan: a fan in front of the user that blows a gust along the
-- enemy's rows every 12 ticks for 24 seconds: Wind's pushes away from the
-- enemy's front column, Fan's pulls from the far column
-- (docs/engine/dimming-chip-effects.md §1).

local controller = require("@self/controller")
local fan = require("@self/fan")

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
    extra_flags = { "free_slot_in" },
    library = { number = 134, index = 134, sort = 510 },
    slot_in_limit = 3,
    dimming = controller.hook(fan.variant {
        palette = 0,
        pulls = false,
        wait = 1,
        hp = 40,
        identity = define.identity {
            id = "wind",
            class = "field_object",
            version = 3,
            ai_index = 1,
            look = { sprite = asset.sprite("fan"), anim = 0, palette = 0, shadow = false },
        },
        absorbed = fan.absorbed,
    }),
}

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
    extra_flags = { "free_slot_in" },
    library = { number = 135, index = 135, sort = 201 },
    slot_in_limit = 3,
    dimming = controller.hook(fan.variant {
        palette = 1,
        pulls = true,
        wait = 1,
        hp = 40,
        identity = define.identity {
            id = "fan",
            class = "field_object",
            version = 4,
            ai_index = 1,
            look = { sprite = asset.sprite("fan"), anim = 0, palette = 1, shadow = false },
        },
        absorbed = fan.absorbed,
    }),
}

return { wind = wind, fan = fan_chip }
