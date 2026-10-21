--!strict
-- The PartyCust program P.Spport (EXE5 DS's item 0xBC: ov413's tables at 0x021EC97C, its apply 0x021E052C's case
-- 12): it fills 1 block of a team navi's gauge, in white.

local partycust = require("../rules/party/partycust")

local program: PartyCustProgram = {
    cost = 1,
    color = "white",
    kind = "special",
    effects = { partycust.support() },
}

return { ["p-spport"] = program }
