--!strict
-- Geddon1, Geddon2 and Geddon3: the geddon series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

local quake = controller.quake

-- Geddon1's quake cracks the field's solid panels (the quake's row 0), Geddon2's breaks them (row 1), a puff over
-- each; Geddon3's poisons them (row 2), bubbles over each.
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

-- Geddon1 (EXE5 chip 0x07b): a quake cracks every solid panel of the field, one every 8 ticks in a random order:
-- EXE6's Geddon (its quake's first row) without holding the user's panel (chips/geddon/controller).
local geddon1: Chip = {
    codes = { "M", "P", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 34,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook(CRACK),
    auto_battle = { class = "at_once" },
}

-- Geddon2 (EXE5 chip 0x07c): a quake breaks every solid panel of the field (cracking those something stands on), one
-- every 8 ticks in a random order: EXE6's Geddon without holding the user's panel (chips/geddon/controller).
local geddon2: Chip = {
    codes = { "H", "T", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 47,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook(BREAK),
    auto_battle = { class = "at_once" },
}

-- Geddon3 (EXE5 chip 0x07d): a quake turns every solid panel of the field to poison, one every 8 ticks in a random
-- order: EXE6's Geddon (its quake's third row, which no EXE6 chip uses) without holding the user's panel
-- (chips/geddon/controller).
local geddon3: Chip = {
    codes = { "A", "O", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 62,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook(POISON),
    auto_battle = { class = "at_once" },
}

return { geddon1 = geddon1, geddon2 = geddon2, geddon3 = geddon3 }
