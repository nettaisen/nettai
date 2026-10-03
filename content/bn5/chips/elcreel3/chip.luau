--!strict
-- ElcReel3 (BN5 chip 0x0a5): the navi launches a reel along its row; what it hits, it hooks with every navi around
-- it, and shocks them: BN5's own (objects/elcreel).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local elcreel = require("../../objects/elcreel/elcreel")

return define.chip {
    id = "bn5:elcreel3",
    codes = { "E", "L", "R" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 60,
    damage = 120,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = elcreel.action,
}
