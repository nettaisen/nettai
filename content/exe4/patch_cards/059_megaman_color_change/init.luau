--!strict
-- The patch card MegaMan Color Change (59, ロックマンカラーを変更).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    effects = {
        cards.color(1),
    },
}

return { ["059-megaman-color-change"] = card }
