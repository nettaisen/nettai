--!strict
-- MagnetMn (BN5 chip 0x0e9): MagnetMan cuts in and sends two magnets sliding along his row to meet at the first enemy
-- navi, immobilizing (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:magnetmn",
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 2,
    mb = 25,
    damage = 90,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon,
}
