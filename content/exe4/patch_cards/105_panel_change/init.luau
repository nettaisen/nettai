--!strict
-- The patch card Panel Change (105, パネルチェンジ).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    effects = {
        cards.panel_trail(9),
        cards.clear_bug("panel"),
        cards.bug("hp", 1),
        cards.bug("custom", 1),
    },
}

return { ["105-panel-change"] = card }
