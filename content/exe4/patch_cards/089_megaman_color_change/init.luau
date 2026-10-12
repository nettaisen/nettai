--!strict
-- The patch card MegaMan Color Change (89, ロックマンカラーを変更), slot 0C's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: SlottedPatchCard = {
    slot = "0C",
    effects = {
        cards.color(3),
    },
}

return { ["089-megaman-color-change"] = card }
