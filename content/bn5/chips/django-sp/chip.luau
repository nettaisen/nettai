--!strict
-- DjangoSP (BN5 chip 0x117): Django cuts in, shuts the first enemy navi ahead in his row in his coffin and purifies
-- it under his sun (../django/navi); its damage (the purification's ticks) by the turn (formula 22, 0x0800E98A: its
-- table's row 21, a step a turn from the first).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("../django/navi")

return define.chip {
    id = "bn5:django-sp",
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 68,
    damage = { formula = "count", of = "turns_before", by_count = { 150, 160, 170, 180, 190, 200, 210, 220, 230, 240, 250 }, per_player_gauges = 200 },
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library", "variable_damage" },
    slot_in_limit = 1,
    megaman = "light",
    navi = navi.summon_sp,
}
