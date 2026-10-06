--!strict
-- The patch card Momogra (65, モモグラン in the Japanese games), 6 MB.

local cards = require("../rules/patch_cards/cards")
local invisible_charge = require("../lib/patch_cards/invisible_charge")

local card: PatchCard = {
    mb = 6,
    effects = {
        cards.bug(cards.hp(-70)),
        cards.charged_shot(invisible_charge),
        cards.rush(true),
    },
}

return { momogra = card }
