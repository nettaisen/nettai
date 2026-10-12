--!strict
-- The patch card Weapon Change (133, ウェポンチェンジ), slot 0A's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: SlottedPatchCard = {
    slot = "0A",
    effects = {
        cards.b_button(chips.minibomb),
    },
}

return { ["133-weapon-change"] = card }
