--!strict
-- The patch card First Barrier 100 (11, ファーストバリア100).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")
local barriers = require("../lib/barriers")

local card: PatchCard = {
    effects = {
        cards.aura(barriers.barrier_100),
        cards.bug("hp", 1),
    },
}

return { ["011-first-barrier-100"] = card }
