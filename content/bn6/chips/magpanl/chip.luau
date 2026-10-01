--!strict
-- MagPanl: a special chip (no folder holds it) whose record runs Geddon's
-- quake (chips/geddon) with its first row, cracking the field's solid
-- panels.

local controller = require("../geddon/controller")

return define.chip {
    id = "magpanl",
    name = "MagPanl",
    description = "ミテイ",
    element = "null", family = "special", class = "special",
    rarity = 0, mb = 99, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming" },
    library = { number = 386, index = 0, sort = 0 },
    slot_in_limit = 1,
    dimming = controller.hook("crack"),
}
