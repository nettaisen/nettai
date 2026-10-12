--!strict
-- The patch card Max HP Up (10, マックスHPを強化), slot 0B's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: SlottedPatchCard = {
    slot = cards.SLOT_0B,
    effects = {
        cards.hp(300),
    },
}

return { ["010-max-hp-up"] = card }
