--!strict
-- The patch card Max HP Up (3, マックスHPを強化).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.hp(150),
    },
}

return { ["003-max-hp-up"] = card }
