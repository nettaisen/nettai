--!strict
-- Mag-Barr (EXE5 chip 0x186, past the named ones): the chip whose name MagnetMan's Mag-Barr's telop
-- (navis/magnetman/support: the DS's 0x02098A84) shows (EXE5 DS's party battle).
-- Its record is the GBA's placeholder: no folder or hand holds it, and its use, the Attck+ chips' (action 0x1A's subtype
-- 3), never runs. Its name is the DS's (chipname1.msg).

local plus = require("../lib/plus")

local chip: Chip = {
    codes = {},
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = 10,
    hit_param = 0,
    lockout = 0,
    flags = {},
    slot_in_limit = 1,
    instant = plus.attack,
}

return { ["mag-barr"] = chip }
