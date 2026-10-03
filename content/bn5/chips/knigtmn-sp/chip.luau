--!strict
-- KnigtMnSP (BN5 chip 0x0ff): KnightMan cuts in and swings his wrecking ball around himself, confusing what it hits
-- (../knightmn/navi); its damage by how fast its user deleted KnightMan SP (rules/sp-chips: formula 12, its table's row
-- 11).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../knightmn/navi")

return define.chip {
    id = "bn5:knigtmn-sp",
    codes = { "K" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 3,
    mb = 90,
    damage = { formula = "sp_navi", slot = "sp/knightmn", by_time = { 320, 310, 300, 290, 280, 270, 260, 250, 240, 230, 220 }, per_player_gauges = 300 },
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
