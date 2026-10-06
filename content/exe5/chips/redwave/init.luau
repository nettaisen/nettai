--!strict
-- RedWave (EXE5 chip 0x0ad): in each row, the first lava panel from the user's back column starts a wave toward the
-- other side, and turns Normal; past the field, the waves raise geysers under the other side's navis: EXE5's own
-- (chips/wavepit/wavepit).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local wavepit = require("./wavepit/wavepit")

local chip: Chip = {
    codes = { "A", "F", "H" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 48,
    damage = 190,
    hit_param = 30,
    lockout = 20,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 3,
    instant = wavepit.instant(wavepit.pits.redwave),
    auto_battle = { class = "when_any_lava" },
}

return { redwave = chip }
