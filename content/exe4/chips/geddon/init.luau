--!strict
-- Geddon1, Geddon2 and Geddon3: the geddon series, the dimming chips' variant 0x0A (each chip's own comment above its
-- definition).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local controller = require("@self/controller")

local quake = controller.quake

-- Geddon1's quake cracks the field's solid panels (the quake's row 0, 0x08009B50), Geddon2's breaks them (row 1,
-- 0x08009CAC), a puff over each; Geddon3's poisons them (row 2, 0x08009D68), bubbles over each.
local CRACK = quake.change {
    apply = function(x: number, y: number): boolean
        return field.crack(x, y)
    end,
    show = quake.puff,
}
local BREAK = quake.change {
    apply = function(x: number, y: number): boolean
        return field.break_panel(x, y)
    end,
    show = quake.puff,
}
local POISON = quake.change { apply = quake.poison, show = quake.bubbles }

-- Geddon1 (EXE4 chip 0x07B): a quake cracks every solid panel of the field, one every 8 ticks in a random order.
local geddon1: Chip = {
    codes = { "J", "K", "W" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 34,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = controller.hook(CRACK),
}

-- Geddon2 (EXE4 chip 0x07C): a quake breaks every solid panel of the field (cracking those something stands on), one
-- every 8 ticks in a random order.
local geddon2: Chip = {
    codes = { "N", "T", "Y" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 47,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = controller.hook(BREAK),
}

-- Geddon3 (EXE4 chip 0x07D): a quake turns every solid panel of the field to poison, one every 8 ticks in a random
-- order.
local geddon3: Chip = {
    codes = { "H", "Q", "U" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 62,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = controller.hook(POISON),
}

return { geddon1 = geddon1, geddon2 = geddon2, geddon3 = geddon3 }
