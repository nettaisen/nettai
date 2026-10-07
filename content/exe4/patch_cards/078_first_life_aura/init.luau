--!strict
-- The patch card First Life Aura (78, ファーストドリームオーラ).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")
local barriers = require("../lib/barriers")

local card: PatchCard = {
    effects = {
        cards.aura(barriers.life_aura),
        cards.bug("hp", 1),
    },
}

return { ["078-first-life-aura"] = card }
