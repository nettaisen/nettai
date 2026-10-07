--!strict
-- The patch card Weapon Change (107, ウェポンチェンジ).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local taunt = require("../navis/megaman/weapons/taunt")

local card: PatchCard = {
    effects = {
        cards.b_button(taunt.weapon),
    },
}

return { ["107-weapon-change"] = card }
