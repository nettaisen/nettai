--!strict
-- The PartyCust program P.HP+200 (EXE5 DS's item 0xB2: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 2): it fills 3 blocks of a team navi's gauge, in red.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 3,
    color = "red",
    kind = "hp",
    effects = { partycust.hp(200) },
}

return { ["p-hp-200"] = program }
