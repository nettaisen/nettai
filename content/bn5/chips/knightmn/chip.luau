--!strict
-- KnightMn (BN5 chip 0x0fe): KnightMan cuts in and swings his wrecking ball around himself, confusing what it hits
-- (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:knightmn",
    codes = { "K" },
    element = "wood",
    family = "break",
    class = "mega",
    rarity = 2,
    mb = 50,
    damage = 170,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
