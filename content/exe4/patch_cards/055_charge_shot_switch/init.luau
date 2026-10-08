--!strict
-- The patch card Charge Shot Switch (55, チャージ武器を変更).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    effects = {
        cards.charged_shot(chips.elecshok),
        cards.clear_bug("weapon"),
    },
}

return { ["055-charge-shot-switch"] = card }
