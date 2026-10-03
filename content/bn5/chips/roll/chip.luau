--!strict
-- Roll (BN5 chip 0x0dd): Roll whips the enemy navi ahead and heals her user with a heart (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:roll",
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 2,
    mb = 20,
    damage = 20,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    -- (AntiRecv springs on her heal; she brings her user back herself.)
    traits = { "heals", "navi_returns_user" },
    navi = navi.summon { rounds = 1, palette = 0, level = 0 },
}
