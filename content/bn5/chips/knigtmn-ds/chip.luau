--!strict
-- KnigtMnDS (BN5 chip 0x100): KnightMan, dark, cuts in and swings his wrecking ball around himself, confusing what it
-- hits (../knightmn/navi); its damage by its user's side's statistic 3 (formula 34, its table's row 11).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../knightmn/navi")

return define.chip {
    id = "bn5:knigtmn-ds",
    codes = { "K" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 3,
    mb = 90,
    damage = { formula = "count", of = { side_stat = 3 }, by_count = { 220, 230, 240, 250, 260, 270, 280, 290, 300, 310, 320 }, per_player_gauges = 250 },
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
    navi = navi.summon_dark,
}
