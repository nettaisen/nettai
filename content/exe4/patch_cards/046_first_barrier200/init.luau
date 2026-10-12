--!strict
-- The patch card First Barrier200 (46, ファーストバリア200), slot 0C's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local barriers = require("../lib/barriers")

local card: SlottedPatchCard = {
    slot = cards.SLOT_0C,
    effects = {
        cards.aura(barriers.barrier_200),
        cards.bug("hp", 1),
    },
}

return { ["046-first-barrier200"] = card }
