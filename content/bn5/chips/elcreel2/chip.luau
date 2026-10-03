--!strict
-- ElcReel2 (BN5 chip 0x0a4): the navi launches a reel along its row; what it hits, it hooks with every navi around
-- it, and shocks them: BN5's own (objects/elcreel).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local elcreel = require("../../objects/elcreel/elcreel")

return define.chip {
    id = "bn5:elcreel2",
    codes = { "B", "Q", "Y" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 46,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = elcreel.action,
}
