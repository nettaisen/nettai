--!strict
-- AntiElec (EXE5 chip 0x08e): a trap the other side's first elec hit springs, striking back in elec on every
-- panel the enemy stands on (lib/traps/anti_trap: EXE6's ElemTrap trap and counterattack, EXE5's).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local anti_trap = require("../lib/traps/anti_trap")

local chip: Chip = {
    codes = { "A", "N", "Y", "*" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 59,
    damage = 200,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = anti_trap.hook(3),
    auto_battle = { class = "when_trap_free" },
}

return { antielec = chip }
