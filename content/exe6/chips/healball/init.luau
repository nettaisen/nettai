--!strict
-- HealBall: a special chip (no folder holds it) whose record runs Geddon's
-- quake (chips/geddon) with its first row, cracking the field's solid
-- panels.

local controller = require("./geddon/controller")
local quake = require("./geddon/quake")

-- Its quake cracks the field's solid panels (the quake's row 0), a puff
-- over each.
local CRACK = quake.change {
    apply = function(x: number, y: number): boolean
        return field.crack(x, y)
    end,
    show = quake.puff,
}

local chip: Chip = {
    element = "null", family = "special", class = "special",
    rarity = 0, mb = 99, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming" },
    slot_in_limit = 1,
    -- (The other side can't cut in on its dimming.)
    traits = { "no_cut_in" },
    dimming = controller.hook(CRACK),
}

return { healball = chip }
