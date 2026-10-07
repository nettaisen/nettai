--!strict
-- The patch card MegaMan Color Change (90, ロックマンカラーを変更).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.color(4),
    },
}

return { ["090-megaman-color-change"] = card }
