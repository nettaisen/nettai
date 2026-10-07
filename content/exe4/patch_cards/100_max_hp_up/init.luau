--!strict
-- The patch card Max HP Up (100, マックスHPを強化).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.hp(950),
        cards.bug("hp", 2),
    },
}

return { ["100-max-hp-up"] = card }
