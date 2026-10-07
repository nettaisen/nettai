--!strict
-- The patch card Max HP Up (5, マックスHPを強化).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    effects = {
        cards.hp(200),
    },
}

return { ["005-max-hp-up"] = card }
