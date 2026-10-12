--!strict
-- The patch card FullCustom Charge 2/2 (104, チャージフルカスタム2/2), slot 0E's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: SlottedPatchCard = {
    slot = "0E",
    effects = {
        cards.pair(3, "103-fullcustom-charge-1-2", { cards.charged_shot(chips.fullcust), cards.custom_down(2) }),
    },
}

return { ["104-fullcustom-charge-2-2"] = card }
