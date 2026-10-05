--!strict
-- The second of two special BeastOut records with a dimming use (no
-- folder holds them; the BeastOut chip of the custom screen is the
-- instant chip `beastout`): Geddon's quake (chips/geddon) with its first
-- row, cracking the field's solid panels.

local controller = require("./geddon/controller")

return define.chip {
    id = "beastout-dimming-2",
    element = "null", family = "special", class = "special",
    rarity = 0, mb = 99, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming" },
    library = { number = 391, index = 0, sort = 0 },
    slot_in_limit = 1,
    -- (The other side can't cut in on its dimming.)
    traits = { "no_cut_in" },
    dimming = controller.hook("crack"),
}
