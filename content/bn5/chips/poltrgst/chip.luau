--!strict
-- Poltrgst (BN5 chip 0x0d5): the user's stand-in raises a poltergeist that throws every obstacle on the field at the
-- other side's navi, front to back, 20 ticks apart: BN5's own (./poltergeist, the dimming handler's subtype 49; its
-- first parameter 0: random targets).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local poltergeist = require("./poltergeist")

return define.chip {
    id = "bn5:poltrgst",
    codes = { "P" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 40,
    damage = 150,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    dimming = poltergeist.hook(false),
}
