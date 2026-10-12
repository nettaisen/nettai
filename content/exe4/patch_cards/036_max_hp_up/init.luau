--!strict
-- The patch card Max HP Up (36, マックスHPを強化), slot 0C's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: SlottedPatchCard = {
    slot = "0C",
    effects = {
        cards.hp(450),
        cards.bug("move", 1),
    },
}

return { ["036-max-hp-up"] = card }
