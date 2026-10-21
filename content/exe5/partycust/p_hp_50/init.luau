--!strict
-- The PartyCust program P.HP+50 (EXE5 DS's item 0xB0: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 0): it fills 1 block of a team navi's gauge.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 1,
    kind = "hp",
    effects = { partycust.hp(50) },
}

return { ["p-hp-50"] = program }
