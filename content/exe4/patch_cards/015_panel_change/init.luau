--!strict
-- The patch card Panel Change (15, パネルチェンジ).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.panel_trail(3),
        cards.clear_bug("panel"),
    },
}

return { ["015-panel-change"] = card }
