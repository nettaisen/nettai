--!strict
-- The PartyCust program P.Atk+1 (EXE5 DS's item 0xB4: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 4): it fills 1 block of a team navi's gauge.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 1,
    kind = "attack",
    effects = { partycust.attack(1) },
}

return { ["p-atk-1"] = program }
