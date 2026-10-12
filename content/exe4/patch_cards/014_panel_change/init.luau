--!strict
-- The patch card Panel Change (14, パネルチェンジ), slot 0D's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: SlottedPatchCard = {
    slot = "0D",
    effects = {
        cards.panel_trail(5),
        cards.clear_bug("panel"),
    },
}

return { ["014-panel-change"] = card }
