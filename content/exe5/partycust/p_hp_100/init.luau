--!strict
-- The PartyCust program P.HP+100 (EXE5 DS's item 0xB1: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 1): it fills 2 blocks of a team navi's gauge, in red.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 2,
    color = "red",
    kind = "hp",
    effects = { partycust.hp(100) },
}

return { ["p-hp-100"] = program }
