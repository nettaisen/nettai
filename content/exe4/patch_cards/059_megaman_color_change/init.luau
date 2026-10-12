--!strict
-- The patch card MegaMan Color Change (59, ロックマンカラーを変更), slot 0C's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: SlottedPatchCard = {
    slot = cards.SLOT_0C,
    effects = {
        cards.color(1),
    },
}

return { ["059-megaman-color-change"] = card }
