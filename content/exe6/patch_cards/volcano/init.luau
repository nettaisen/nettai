--!strict
-- The patch card Volcano (13, ボルカノ in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 10,
    effects = {
        cards.body("fire"),
        cards.charge(4),
        cards.charged_shot(charge),
        cards.bug(cards.emotion_bug(1)),
    },
}

return { volcano = card }
