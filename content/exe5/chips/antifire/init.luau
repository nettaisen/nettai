--!strict
-- AntiFire (EXE5 chip 0x08c): a trap the other side's first fire hit springs, striking back in fire on every
-- panel the enemy stands on (lib/traps/anti_trap: EXE6's ElemTrap trap and counterattack, EXE5's).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local anti_trap = require("../lib/traps/anti_trap")

local chip: Chip = {
    codes = { "E", "K", "T", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 58,
    damage = 200,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = anti_trap.hook(1),
    auto_battle = { class = "when_trap_free", element_trap = true },
}

return { antifire = chip }
