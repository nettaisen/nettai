--!strict
-- The patch card Max HP Up (95, マックスHPを強化).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.hp(800),
        cards.bug("panel", 3),
    },
}

return { ["095-max-hp-up"] = card }
