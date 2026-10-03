--!strict
-- SideBub3 (BN5 chip 0x0a2): the navi sends two bubbles to the column ahead, in the rows above and below its own,
-- drifting forward and bouncing between the top and bottom rows; a popped bubble bursts into an aqua shot: BN5's
-- own (objects/sidebub).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local sidebub = require("../../objects/sidebub/sidebub")

return define.chip {
    id = "bn5:sidebub3",
    codes = { "E", "N", "P" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 45,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = sidebub.action,
}
