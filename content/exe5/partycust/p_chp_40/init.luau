--!strict
-- The PartyCust program P.Chp+40 (EXE5 DS's item 0xB8: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 8): it fills 4 blocks of a team navi's gauge, in blue.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 4,
    color = "blue",
    kind = "chip_attack",
    effects = { partycust.chip_attack(40) },
}

return { ["p-chp-40"] = program }
