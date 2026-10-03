--!strict
-- DarkTorn (BN5 chip 0x0c5): a dark chip: action 0x22's third form, HP-bug tornadoes spreading back by the side's
-- statistic 3 (objects/tornado). Its use costs custom-screen damage (rules/light-dark).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local tornado = require("../../objects/tornado/tornado")

return define.chip {
    id = "bn5:darktorn",
    codes = { "T" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 50,
    hit_param = 132,
    lockout = 0,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    action = tornado.back_spread,
}
