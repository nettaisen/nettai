--!strict
-- RollSP (BN5 chip 0x0de): Roll (../roll/navi), turning her side's holy panels Normal as she comes; its damage by
-- the holy panels of its user's side (formula 1, 0x0800E8B4).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../roll/navi")

return define.chip {
    id = "bn5:roll-sp",
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 3,
    mb = 60,
    damage = { formula = "count", of = { own_panels = "holy" }, by_count = { 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80 }, per_player_gauges = 50 },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "light",
    -- (AntiRecv springs on her heal; she brings her user back herself.)
    traits = { "heals", "navi_returns_user" },
    navi = navi.summon { rounds = 1, palette = 0, level = 3 },
}
