--!strict
-- HPRecov (EXE5 chip 0x187, past the named ones): the chip whose name ToadMan's rescue's telop shows (EXE5 DS's party
-- battle, navis/toadman/rescue: its telop 0x187, 0x02004938). Its record is the GBA's placeholder (the DS's the same
-- but its +0x1E): no folder or hand holds it, and its use, the Attck+ chips' (action 0x1A's subtype 3), never runs.
-- Its name is the DS's (chipname1.msg).

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

return { hprecov = chip }
