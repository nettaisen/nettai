--!strict
-- CrossDiv (EXE5 chip 0x135): Colonel cuts in and divides the screen with an X on the first enemy navi ahead
-- (../colonel/navi, his Param3 1).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("./colonel/navi")

local chip: Chip = {
    codes = { "C" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 90,
    damage = 220,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    extra_flags = { 8, 16 },
    slot_in_limit = 1,
    navi = navi.summon_cross,
    auto_battle = { class = "at_once" },
}

return { crossdiv = chip }
