--!strict
-- The snowball's roller (the original's attack object #0xB7, `sub_80DC260`):
-- the shared roller (@exelib/blzrdbal/roller), EXE6's kind: what it is and
-- reaches (rows 0x58 and 0x49), every attack, object and body of either
-- side; hit modifier 0, no spark.

local common = require("@exelib/blzrdbal/roller")

local roller = {}

roller.kind = common.make_kind {
    id = "blzrdbal/roller",
    self_type = define.collision { side0 = 0xC0044080, side1 = 0xC0044080, row_offset = 0x2C0 },
    target = define.collision {
        side0 = 0x3F800000,
        side1 = 0x3F800000,
        row_offset = 0x248,
    },
}

return roller
