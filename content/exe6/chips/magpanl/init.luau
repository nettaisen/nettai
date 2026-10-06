--!strict
-- MagPanl: a special chip (no folder holds it) whose record runs Geddon's
-- quake (chips/geddon) with its first row, cracking the field's solid
-- panels.

local controller = require("./geddon/controller")

return define.chip {
    id = "magpanl",
    element = "null", family = "special", class = "special",
    rarity = 0, mb = 99, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming" },
    slot_in_limit = 1,
    -- (The other side can't cut in on its dimming.)
    traits = { "no_cut_in" },
    dimming = controller.hook("crack"),
}
