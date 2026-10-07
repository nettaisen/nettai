--!strict
-- The patch card Custom2 (17, カスタム2).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.custom(2),
        cards.bug("custom", 1),
    },
}

return { ["017-custom2"] = card }
