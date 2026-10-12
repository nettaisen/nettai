--!strict
-- The patch card First Life Aura (78, ファーストドリームオーラ), slot 0C's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local barriers = require("../lib/barriers")

local card: SlottedPatchCard = {
    slot = "0C",
    effects = {
        cards.aura(barriers.life_aura),
        cards.bug("hp", 1),
    },
}

return { ["078-first-life-aura"] = card }
