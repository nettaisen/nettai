--!strict
-- The patch card Max HP Up (62, マックスHPを強化), slot 0D's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: SlottedPatchCard = {
    slot = "0D",
    effects = {
        cards.hp(550),
    },
}

return { ["062-max-hp-up"] = card }
