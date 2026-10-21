--!strict
-- The PartyCust program P.Atk+3 (EXE5 DS's item 0xB6: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 6): it fills 3 blocks of a team navi's gauge, in yellow.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 3,
    color = "yellow",
    kind = "attack",
    effects = { partycust.attack(3) },
}

return { ["p-atk-3"] = program }
