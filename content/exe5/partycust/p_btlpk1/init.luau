--!strict
-- The PartyCust program P.Btlpk1 (EXE5 DS's item 0xBA: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 10): it fills 4 blocks of a team navi's gauge.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 4,
    kind = "special",
    effects = { partycust.hp(50), partycust.attack(1), partycust.chip_attack(30) },
}

return { ["p-btlpk1"] = program }
