--!strict
-- The patch card Panel Change (14, パネルチェンジ).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.panel_trail(5),
        cards.clear_bug("panel"),
    },
}

return { ["014-panel-change"] = card }
