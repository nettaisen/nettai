--!strict
-- The patch card MegaFolder2 (18, メガフォルダ2).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    effects = {
        cards.mega_folder(2),
        cards.custom_down(1),
    },
}

return { ["018-megafolder2"] = card }
