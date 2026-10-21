--!strict
-- The PartyCust program P.HP+300 (EXE5 DS's item 0xB3: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 3): it fills 5 blocks of a team navi's gauge.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 5,
    kind = "hp",
    effects = { partycust.hp(300) },
}

return { ["p-hp-300"] = program }
