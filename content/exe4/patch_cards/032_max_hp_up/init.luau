--!strict
-- The patch card Max HP Up (32, マックスHPを強化), slot 0C's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: SlottedPatchCard = {
    slot = cards.SLOT_0C,
    effects = {
        cards.hp(350),
    },
}

return { ["032-max-hp-up"] = card }
