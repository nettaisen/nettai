--!strict
-- The patch card Triple Supporter 2/2 (75, トリプルサポーター2/2).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    effects = {
        cards.pair(1, "074-triple-supporter-1-2", { cards.all_supports(), cards.clear_bug("support") }),
    },
}

return { ["075-triple-supporter-2-2"] = card }
