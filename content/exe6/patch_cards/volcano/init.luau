--!strict
-- The patch card Volcano (13, ボルカノ in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

return define.patch_card {
    id = "volcano",
    mb = 10,
    effects = {
        cards.body("fire"),
        cards.charge(4),
        cards.charged_shot(charge),
        cards.bug(cards.emotion_bug(1)),
    },
}
