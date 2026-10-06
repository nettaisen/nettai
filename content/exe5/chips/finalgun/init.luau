--!strict
-- FinalGun (EXE5 chip 0x122): one of the Liberation Missions' mode chips: instant chip effect 11 (0x0802D62A, EXE6's
-- `sub_802E1BE`, @exelib/instant/side_special; the same code) writes its mode (the record's first parameter) and a
-- 480-tick timer (360 in the operation battle) to the user's side state, which nothing in a battle reads (0x0800E576
-- counts the timer down), so the chip's use changes nothing observable.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local side_special = require("@exelib/instant/side_special")

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 80,
    damage = 50,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage" },
    extra_flags = { 16, 32 },
    slot_in_limit = 1,
    instant = side_special.instant,
    auto_battle = { class = "at_once" },
}

return { finalgun = chip }
