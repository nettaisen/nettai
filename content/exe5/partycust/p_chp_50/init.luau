--!strict
-- The PartyCust program P.Chp+50 (EXE5 DS's item 0xB9: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 9): it fills 5 blocks of a team navi's gauge.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 5,
    kind = "chip_attack",
    effects = { partycust.chip_attack(50) },
}

return { ["p-chp-50"] = program }
