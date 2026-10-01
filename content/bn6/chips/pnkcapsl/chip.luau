--!strict
-- PnkCapsl: a special chip (no folder holds it) whose dimming sets off
-- Geddon's quake (chips/geddon), breaking the field's solid panels. It
-- dims without the dimming flag, so it can't be used as a cut-in.

local controller = require("../geddon/controller")

return define.chip {
    id = "pnkcapsl",
    name = "PnkCapsl",
    codes = {},
    element = "null", family = "summon", class = "special",
    rarity = 0, mb = 6, damage = 0, hit_param = 0, lockout = 0,
    library = { number = 384, index = 0, sort = 0 },
    slot_in_limit = 1,
    dimming = controller.hook("break"),
}
