--!strict
-- WavePit (EXE5 chip 0x0ac): in each row, the first sea panel from the user's back column starts a wave toward the
-- other side, and turns Normal; past the field, the waves raise geysers under the other side's navis: EXE5's own
-- (chips/wavepit/wavepit).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local wavepit = require("@self/wavepit")

local chip: Chip = {
    codes = { "B", "E", "Z" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 42,
    damage = 160,
    hit_param = 30,
    lockout = 20,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 3,
    -- Its pit (row 0 of 0x080DAFA0): a sea panel (flag 0x20000), which turns Normal; the wave in palette 0.
    instant = wavepit.instant { require = 0x0002_0000, forbid = 0, mends = true, palette = 0 },
    auto_battle = { class = "when_any_sea" },
}

return { wavepit = chip }
