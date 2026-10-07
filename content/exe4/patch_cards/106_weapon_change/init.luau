--!strict
-- The patch card Weapon Change (106, ウェポンチェンジ).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local guards = require("../navis/megaman/weapons/guards")

local card: PatchCard = {
    effects = {
        cards.b_button(guards.reflect),
        cards.bug("hp", 2),
    },
}

return { ["106-weapon-change"] = card }
