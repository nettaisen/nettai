--!strict
-- PnkCapsl: a special chip (no folder holds it) whose dimming sets off
-- Geddon's quake (chips/geddon), breaking the field's solid panels. It
-- dims without the dimming flag, so it can't be used as a cut-in.

local controller = require("./geddon/controller")
local quake = require("./geddon/quake")

-- Its quake breaks the field's solid panels (the quake's row 1), a puff
-- over each.
local BREAK = quake.change {
    apply = function(x: number, y: number): boolean
        return field.break_panel(x, y)
    end,
    show = quake.puff,
}

local chip: Chip = {
    element = "null", family = "summon", class = "special",
    rarity = 0, mb = 6, damage = 0, hit_param = 0, lockout = 0,
    slot_in_limit = 1,
    -- (The other side can't cut in on its dimming.)
    traits = { "no_cut_in" },
    dimming = controller.hook(BREAK),
}

return { pnkcapsl = chip }
