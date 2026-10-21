--!strict
-- The PartyCust program P.Chp+30 (EXE5 DS's item 0xB7: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 7): it fills 3 blocks of a team navi's gauge.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 3,
    kind = "chip_attack",
    effects = { partycust.chip_attack(30) },
}

return { ["p-chp-30"] = program }
