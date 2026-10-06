--!strict
-- BigHook (EXE5 chip 0x134): The hook swings two big flames across the enemy area (./hook); its user stays on the field.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local hook = require("@self/hook")

local chip: Chip = {
    codes = { "G" },
    element = "null",
    family = "break",
    class = "giga",
    rarity = 4,
    mb = 92,
    damage = 240,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    extra_flags = { 8, 16 },
    slot_in_limit = 1,
    megaman = "light",
    -- (Its user stays on the field while the hook strikes: 0x080E1910, navi 26.)
    traits = { "user_stays" },
    navi = hook.summon,
}

return { bighook = chip }
