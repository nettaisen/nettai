--!strict
-- MudWave (BN5 chip 0x0ae): in each row, the first hole from the user's back column starts a wave toward the other
-- side; past the field, the waves raise geysers under the other side's navis: BN5's own (chips/wavepit/wavepit).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local wavepit = require("./wavepit/wavepit")

return define.chip {
    id = "mudwave",
    codes = { "D", "N", "W" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 3,
    mb = 54,
    damage = 170,
    hit_param = 30,
    lockout = 20,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 3,
    instant = wavepit.instant(wavepit.pits.mudwave),
}
