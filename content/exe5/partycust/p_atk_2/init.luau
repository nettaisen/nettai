--!strict
-- The PartyCust program P.Atk+2 (EXE5 DS's item 0xB5: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 5): it fills 2 blocks of a team navi's gauge, in yellow.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 2,
    color = "yellow",
    kind = "attack",
    effects = { partycust.attack(2) },
}

return { ["p-atk-2"] = program }
