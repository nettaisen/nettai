--!strict
-- NapalmMn (BN5 chip 0x0e3): NapalmMan cuts in and lobs a napalm bomb four panels ahead, its fire spreading around an
-- empty panel (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:napalmmn",
    codes = { "N" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 2,
    mb = 38,
    damage = 120,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
