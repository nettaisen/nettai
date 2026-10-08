--!strict
-- The patch card Weapon Change (108, ウェポンチェンジ).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local bug_shots = require("../navis/megaman/weapons/bug_shots")

local card: PatchCard = {
    effects = {
        cards.b_button(bug_shots.bubble),
    },
}

return { ["108-weapon-change"] = card }
