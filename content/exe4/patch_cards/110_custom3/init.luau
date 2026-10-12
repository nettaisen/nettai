--!strict
-- The patch card Custom3 (110, カスタム3), slot 0D's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: SlottedPatchCard = {
    slot = "0D",
    effects = {
        cards.custom(3),
        cards.bug("custom", 1),
    },
}

return { ["110-custom3"] = card }
