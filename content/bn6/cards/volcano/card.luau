--!strict
-- The patch card Volcano (13, ボルカノ in the Japanese games), 10 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/volcano",
    mb = 10,
    effects = {
        cards.body("fire"),
        cards.charge(4),
        cards.charged_shot(charge),
        cards.bug(cards.emotion_bug(1)),
    },
}
