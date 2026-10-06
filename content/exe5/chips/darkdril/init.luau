--!strict
-- DarkDril (EXE5 chip 0x0c4): a dark chip: DrilArm's drill arm and two drills on the two panels ahead, for 60
-- ticks, the arm in its second palette and each drill's hit carrying the bug 0xFA (chips/drilarm/drill). Its use costs a
-- panel trail (rules/light_dark).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local drill = require("./drilarm/drill")

local chip: Chip = {
    codes = { "D" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 100,
    hit_param = 132,
    lockout = 0,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    action = drill.action(true),
}

return { darkdril = chip }
