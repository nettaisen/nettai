--!strict
-- The patch card Shadow (8, ダークシャドー in the Japanese games), 6 MB.

local cards = require("../rules/patch_cards/cards")
local b_sword = require("../lib/patch_cards/b_sword")
local invisible_charge = require("../lib/patch_cards/invisible_charge")

local card: PatchCard = {
    mb = 6,
    effects = {
        cards.bug(cards.status_guard(false)),
        cards.b_button(b_sword),
        cards.charged_shot(invisible_charge),
    },
}

return { shadow = card }
