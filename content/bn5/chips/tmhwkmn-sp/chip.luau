--!strict
-- TmhwkMnSP (BN5 chip 0x0fc): TomahawkMan cuts in and brings his tomahawk down on the two columns ahead
-- (../tmhwkman/navi); its damage by how fast its user deleted TomahawkMan SP (rules/sp-chips: formula 11, its table's
-- row 10).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../tmhwkman/navi")

return define.chip {
    id = "bn5:tmhwkmn-sp",
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 3,
    mb = 81,
    damage = { formula = "sp_navi", slot = "sp/tmhwkman", by_time = { 280, 270, 260, 250, 240, 230, 220, 210, 200, 190, 180 }, per_player_gauges = 250 },
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
