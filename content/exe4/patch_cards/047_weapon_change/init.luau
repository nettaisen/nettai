--!strict
-- The patch card Weapon Change (47, ウェポンチェンジ).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    effects = {
        cards.b_button(chips.guard1),
    },
}

return { ["047-weapon-change"] = card }
