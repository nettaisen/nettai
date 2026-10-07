--!strict
-- The patch card Default Full Synchro (45, デフォルトフルシンクロ).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.full_synchro(),
    },
}

return { ["045-default-full-synchro"] = card }
