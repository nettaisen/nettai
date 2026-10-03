--!strict
-- YoYoMode (BN5 chip 0x11c): one of the Liberation Missions' mode chips: instant chip effect 11 (0x0802D62A, BN6's
-- `sub_802E1BE`, @bn6/lib/instant/side_special; the same code) writes its mode (the record's first parameter) and a
-- 480-tick timer (360 with per-player gauges) to the user's side state, which nothing in a battle reads (0x0800E576
-- counts the timer down), so the chip's use changes nothing observable.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local side_special = require("@bn6/lib/instant/side_special")

return define.chip {
    id = "bn5:yoyomode",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 50,
    damage = 50,
    hit_param = 10,
    lockout = 20,
    flags = {},
    extra_flags = { 32 },
    slot_in_limit = 1,
    instant = side_special.instant,
}
