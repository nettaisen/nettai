--!strict
-- The PartyCust program P.Btlpk2 (EXE5 DS's item 0xBB: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 11): it fills 7 blocks of a team navi's gauge.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 7,
    kind = "special",
    effects = { partycust.hp(200), partycust.attack(2), partycust.chip_attack(40) },
}

return { ["p-btlpk2"] = program }
