--!strict
-- Geddon: a cut-in chip whose quake breaks every solid panel of the field
-- but the user's, one every 8 ticks in a random order (cracking those
-- something stands on) (docs/engine/dimming-chip-effects.md §12).

local controller = require("@self/controller")
local quake = require("@self/quake")

-- Its quake breaks the field's solid panels (the quake's row 1), a puff
-- over each.
local BREAK = quake.change {
    apply = function(x: number, y: number): boolean
        return field.break_panel(x, y)
    end,
    show = quake.puff,
}

local chip: Chip = {
    codes = { "A", "L", "R", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 2, mb = 47, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    dimming = controller.hook(BREAK),
}

return { geddon = chip }
