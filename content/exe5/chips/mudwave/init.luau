--!strict
-- MudWave (EXE5 chip 0x0ae): in each row, the first hole from the user's back column starts a wave toward the other
-- side; past the field, the waves raise geysers under the other side's navis: EXE5's own (chips/wavepit/wavepit).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local wavepit = require("./wavepit/wavepit")

local chip: Chip = {
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
    -- Its pit (row 2 of 0x080DAFA0): a hole (no standable flag 0x10), which stays; the wave in palette 2.
    instant = wavepit.instant { require = 0, forbid = 0x10, mends = false, palette = 2 },
    auto_battle = { class = "when_any_hole" },
}

return { mudwave = chip }
