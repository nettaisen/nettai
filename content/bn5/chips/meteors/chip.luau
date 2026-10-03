--!strict
-- Meteors (BN5 chip 0x0d2): thirty meteors fall on the enemy's area, one every 10 ticks, cycling through its
-- panels from the far column: BN6's Meteors (@bn6/chips/meteors/controller, dimming subtype 18, its meteors' row 1)
-- with BN5's meteor (objects/meteors).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("@bn6/chips/meteors/controller")
local meteors = require("../../objects/meteors/meteors")

local METEORS = controller.make("bn5:meteors/controller", meteors.kind)

return define.chip {
    id = "bn5:meteors",
    codes = { "R" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 4,
    mb = 73,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    dimming = METEORS,
}
