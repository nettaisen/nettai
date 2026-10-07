--!strict
-- The patch card Custom3 (110, カスタム3).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    effects = {
        cards.custom(3),
        cards.bug("custom", 1),
    },
}

return { ["110-custom3"] = card }
