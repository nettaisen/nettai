--!strict
-- The patch card Number Soul (128, ナンバーソウル).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local numbersoul = require("../navis/megaman/forms/numbersoul").numbersoul

local card: PatchCard = {
    effects = {
        cards.soul(numbersoul),
        cards.bug("hp", 2),
    },
}

return { ["128-number-soul"] = card }
