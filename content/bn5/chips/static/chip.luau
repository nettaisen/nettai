--!strict
-- Static (BN5 chip 0x01b): action 0x22, BN6's blow with BN5's fan and tornado (objects/tornado): static tornadoes two
-- panels ahead and around, more the more kinds of NaviCust bug the navi has, paralyzing by that level.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local tornado = require("../../objects/tornado/tornado")

return define.chip {
    id = "bn5:static",
    codes = { "N", "S", "V" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "dark",
    action = tornado.static,
}
